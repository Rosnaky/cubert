use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{sync::mpsc, time::interval};

use crate::{
    broker::alpaca::AlpacaApiBroker,
    config::{RiskConfig, StrategyConfig},
    data::MarketData,
    logging::Logger,
    risk::RiskManager,
    storage::Storage,
    strategy::{Strategy, StrategyParams, StrategySettings, create_strategy},
    types::{Side, Signal},
};

#[derive(Debug)]
pub struct SignalMessage {
    pub strategy_name: String,
    pub signal: Signal,
}

/// Shared data accessible by any component
pub struct SharedState {
    pub storage: Storage,
    pub broker: AlpacaApiBroker,
    pub data: MarketData,
    pub logger: Arc<Logger>,
}

/// Main trading engine
pub struct Engine {
    state: Arc<SharedState>,
    strategies: Vec<Box<dyn Strategy>>,
    risk_manager: RiskManager,
    signal_tx: mpsc::Sender<SignalMessage>,
    signal_rx: mpsc::Receiver<SignalMessage>,
    strategy_configs: Vec<StrategySettings>,
}

impl Engine {
    pub async fn new(
        storage: Storage,
        broker: AlpacaApiBroker,
        data: MarketData,
        logger: Arc<Logger>,
        risk_config: RiskConfig,
        strategy_configs: Vec<StrategySettings>,
    ) -> Self {
        let (signal_tx, signal_rx) = mpsc::channel(100);

        let state = Arc::new(SharedState {
            storage,
            broker,
            data,
            logger,
        });

        Self {
            state,
            strategies: Vec::new(),
            risk_manager: RiskManager::new(risk_config),
            signal_tx,
            signal_rx,
            strategy_configs,
        }
    }

    /// Register a strategy
    pub fn add_strategy(&mut self, config: StrategySettings) {
        let strategy = create_strategy(&config);
        self.state.logger.info(&format!(
            "Registered strategy: {} for {:?}",
            strategy.name(),
            strategy.symbols()
        ));
        self.strategies.push(strategy);
    }

    /// Main loop
    pub async fn run(&mut self, poll_interval_secs: u64) {
        self.state.logger.info("Engine starting...");

        let mut all_symbols: Vec<String> = Vec::new();
        for strategy in &self.strategies {
            for symbol in strategy.symbols() {
                if !all_symbols.contains(symbol) {
                    all_symbols.push(symbol.clone());
                }
            }
        }

        self.state
            .logger
            .info(&format!("Tracking symbols: {:?}", all_symbols));

        // Prefetch historical data
        self.fetch_historical_data(&all_symbols).await;

        let mut ticker = interval(Duration::from_secs(poll_interval_secs));

        loop {
            self.state.logger.info("=== Tick starting ===");

            // Fetch latest data for all symbols
            let mut bars = HashMap::new();
            for symbol in &all_symbols {
                self.state.logger.info(&format!("Fetching {}...", symbol));
                match self.state.data.get_latest_bar(symbol).await {
                    Ok(bar) => {
                        self.state
                            .logger
                            .info(&format!("{}: ${:.2}", symbol, bar.close));
                        bars.insert(symbol.clone(), bar);
                    }
                    Err(e) => {
                        self.state
                            .logger
                            .error(&format!("Failed to fetch {}: {}", symbol, e));
                    }
                }
            }

            // Collect signals first
            let mut signals: Vec<(String, Signal, f64)> = Vec::new();

            for strategy in &mut self.strategies {
                for symbol in strategy.symbols().to_vec() {
                    if let Some(bar) = bars.get(&symbol) {
                        if let Some(signal) = strategy.on_bar(bar) {
                            match &signal {
                                Signal::Buy { .. } | Signal::Sell { .. } => {
                                    signals.push((strategy.name().to_string(), signal, bar.close));
                                }
                                Signal::Hold => {}
                            }
                        }
                    }
                }
            }

            self.state
                .logger
                .info(&format!("Generated {} signals", signals.len()));

            // Now process signals
            for (strategy_name, signal, price) in signals {
                self.process_signal(&strategy_name, &signal, price).await;
            }

            // Save account snapshot
            match self.state.storage.get_account().await {
                Ok(account) => {
                    let _ = self
                        .state
                        .storage
                        .insert_account_snapshot(account.equity, account.cash, account.buying_power)
                        .await;
                    self.state
                        .logger
                        .info(&format!("Account: ${:.2}", account.equity));
                }
                Err(e) => {
                    self.state
                        .logger
                        .error(&format!("Failed to fetch account: {}", e));
                }
            }

            self.state.logger.info(&format!(
                "=== Tick complete. Waiting {} secs ===",
                poll_interval_secs
            ));
            ticker.tick().await;
        }
    }

    async fn process_signal(&mut self, strategy_name: &str, signal: &Signal, current_price: f64) {
        // Log signal
        match signal {
            Signal::Buy { symbol, strength } => {
                self.state.logger.info(&format!(
                    "[{}] BUY signal for {} (strength: {:.2})",
                    strategy_name, symbol, strength
                ));
            }
            Signal::Sell { symbol, strength } => {
                self.state.logger.info(&format!(
                    "[{}] SELL signal for {} (strength: {:.2})",
                    strategy_name, symbol, strength
                ));
            }
            Signal::Hold => return,
        }

        // Save signal to DB
        let (symbol, signal_type, strength) = match signal {
            Signal::Buy { symbol, strength } => (symbol.clone(), "buy", *strength),
            Signal::Sell { symbol, strength } => (symbol.clone(), "sell", *strength),
            Signal::Hold => return,
        };

        let _ = self
            .state
            .storage
            .insert_signal(&symbol, signal_type, strength, strategy_name)
            .await;

        // Get account and positions for risk check
        let account = match self.state.storage.get_account().await {
            Ok(a) => a,
            Err(e) => {
                self.state
                    .logger
                    .error(&format!("Failed to fetch account: {}", e));
                return;
            }
        };

        let positions = match self.state.storage.get_positions().await {
            Ok(p) => p,
            Err(e) => {
                self.state
                    .logger
                    .error(&format!("Failed to fetch positions: {}", e));
                return;
            }
        };

        // Run through risk manager
        if let Some(order) =
            self.risk_manager
                .evaluate_signal(signal, &account, &positions, current_price)
        {
            self.state.logger.info(&format!(
                "Executing order: {:?} {} x {}",
                order.side, order.symbol, order.quantity
            ));

            // Execute order
            match self.state.broker.submit_order(&order).await {
                Ok(order_id) => {
                    self.state
                        .logger
                        .info(&format!("Order filled: {}", order_id));

                    // Save trade
                    let side = match order.side {
                        Side::Buy => "buy",
                        Side::Sell => "sell",
                    };

                    let _ = self
                        .state
                        .storage
                        .insert_trade(
                            &order.symbol,
                            side,
                            order.quantity,
                            current_price,
                            Some(&order_id),
                            Some(strategy_name),
                        )
                        .await;
                }
                Err(e) => {
                    self.state.logger.error(&format!("Order failed: {}", e));
                }
            }
        }
    }

    async fn fetch_historical_data(&mut self, symbols: &Vec<String>) {
        self.state
            .logger
            .info("Fetching historical data for all symbols...");

        for symbol in symbols {
            // Find the strategy config for this symbol
            let config = match self
                .strategy_configs
                .iter()
                .find(|c| c.symbols.contains(symbol))
            {
                Some(c) => c,
                None => {
                    self.state
                        .logger
                        .error(&format!("No strategy config for {}", symbol));
                    continue;
                }
            };

            // Extract startup params from strategy params
            let (timeframe, bar_limit) = match &config.params {
                StrategyParams::Momentum {
                    startup_lookback,
                    startup_bar_limit,
                    ..
                } => (startup_lookback.as_str(), *startup_bar_limit),
                StrategyParams::MeanReversion { .. } => {
                    ("1Hour", 50) // Default fallback
                }
                StrategyParams::Custom { params } => {
                    ("1Hour", 50) // Default fallback
                }
            };

            match self.state.data.get_bars(symbol, timeframe, bar_limit).await {
                Ok(bars) => {
                    self.state.logger.info(&format!(
                        "{}: loaded {} bars ({})",
                        symbol,
                        bars.len(),
                        timeframe
                    ));

                    for bar in &bars {
                        for strategy in &mut self.strategies {
                            if strategy.symbols().contains(symbol) {
                                let _ = strategy.on_bar(bar);
                            }
                        }
                    }
                }
                Err(e) => {
                    self.state
                        .logger
                        .error(&format!("Failed to load history for {}: {}", symbol, e));
                }
            }
        }

        self.state.logger.info("Historical data warmup complete");
    }
}
