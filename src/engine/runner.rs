use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{sync::mpsc, time::interval};

use crate::{
    broker::Broker,
    config::RiskConfig,
    data::MarketData,
    logging::Logger,
    risk::RiskManager,
    storage::Storage,
    strategy::{Strategy, StrategySettings, create_strategy},
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
    pub broker: Arc<dyn Broker>,
    pub data: MarketData,
    pub logger: Arc<Logger>,
}

/// Main trading engine
pub struct Engine {
    state: Arc<SharedState>,
    accounts: HashMap<String, Vec<Box<dyn Strategy>>>,
    risk_manager: RiskManager,
    _signal_tx: mpsc::Sender<SignalMessage>,
    _signal_rx: mpsc::Receiver<SignalMessage>,
}

impl Engine {
    pub async fn new(
        storage: Storage,
        broker: Arc<dyn Broker>,
        data: MarketData,
        logger: Arc<Logger>,
        risk_config: RiskConfig,
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
            accounts: HashMap::new(),
            risk_manager: RiskManager::new(risk_config),
            _signal_tx: signal_tx,
            _signal_rx: signal_rx,
        }
    }

    /// Load strategies from DB for a given account
    pub async fn load_strategies(&mut self, account_id: &str) -> Result<(), String> {
        let assignments = self
            .state
            .storage
            .get_strategies_for_account(account_id)
            .await
            .map_err(|e| format!("Failed to load strategies: {}", e))?;

        let mut strategies: Vec<Box<dyn Strategy>> = Vec::new();

        for (db_strat, assignment) in assignments {
            let params = db_strat
                .params()
                .map_err(|e| format!("Bad params for '{}': {}", db_strat.name, e))?;

            let symbols: Vec<String> =
                serde_json::from_str(&assignment.symbols_json).unwrap_or_default();

            let settings = StrategySettings {
                name: db_strat.name.clone(),
                symbols,
                params,
            };

            let strategy = create_strategy(&settings);
            self.state.logger.info(&format!(
                "Loaded strategy: {} for {:?}",
                strategy.name(),
                strategy.symbols(),
            ));
            strategies.push(strategy);
        }

        self.state.logger.info(&format!(
            "[{}] {} strategies loaded",
            account_id,
            strategies.len()
        ));
        self.accounts.insert(account_id.to_string(), strategies);
        Ok(())
    }

    pub async fn load_all_accounts(&mut self) -> Result<(), String> {
        let account_ids = self
            .state
            .storage
            .get_active_account_ids()
            .await
            .map_err(|e| format!("Failed to load accounts: {}", e))?;

        self.state
            .logger
            .info(&format!("Found {} active accounts", account_ids.len()));

        for account_id in &account_ids {
            self.load_strategies(account_id).await?;
        }

        Ok(())
    }

    pub async fn reload_all(&mut self) -> Result<(), String> {
        self.accounts.clear();
        self.load_all_accounts().await
    }

    fn all_symbols(&self) -> Vec<String> {
        let mut symbols = Vec::new();
        for strategies in self.accounts.values() {
            for strategy in strategies {
                for symbol in strategy.symbols() {
                    if !symbols.contains(symbol) {
                        symbols.push(symbol.clone());
                    }
                }
            }
        }
        symbols
    }

    /// Main loop
    pub async fn run(&mut self, poll_interval_secs: u64) {
        self.state.logger.info("Engine starting...");

        let all_symbols: Vec<String> = self.all_symbols();
        self.state.logger.info(&format!(
            "Tracking {} symbols across {} accounts: {:?}",
            all_symbols.len(),
            self.accounts.len(),
            all_symbols
        ));

        // Prefetch historical data
        self.fetch_historical_data(&all_symbols).await;

        let mut ticker = interval(Duration::from_secs(poll_interval_secs));

        loop {
            self.state.logger.info("=== Tick starting ===");

            self.state.broker.update_prices(&all_symbols).await.ok();

            // Fetch latest data for all symbols
            self.state.logger.info("Fetching latest bars...");
            let bars = match self.state.data.get_latest_bars(&all_symbols).await {
                Ok(b) => {
                    for (symbol, bar) in &b {
                        self.state
                            .logger
                            .info(&format!("{}: ${:.2}", symbol, bar.close));
                    }
                    b
                }
                Err(e) => {
                    self.state
                        .logger
                        .error(&format!("Failed to fetch bars: {}", e));
                    HashMap::new()
                }
            };

            // Collect signals first
            let mut all_signals: Vec<(String, String, Signal, f64)> = Vec::new();

            for (account_id, strategies) in &mut self.accounts {
                for strategy in strategies.iter_mut() {
                    for symbol in strategy.symbols().to_vec() {
                        if let Some(bar) = bars.get(&symbol)
                            && let Some(signal) = strategy.on_bar(bar)
                        {
                            match &signal {
                                Signal::Buy { .. } | Signal::Sell { .. } => {
                                    all_signals.push((
                                        account_id.clone(),
                                        strategy.name().to_string(),
                                        signal,
                                        bar.close,
                                    ));
                                }
                                Signal::Hold => {}
                            }
                        }
                    }
                }
            }

            self.state.logger.info(&format!(
                "Generated {} signals across {} accounts",
                all_signals.len(),
                self.accounts.len(),
            ));

            // Now process signals
            for (account_id, strategy_name, signal, price) in all_signals {
                self.process_signal(&account_id, &strategy_name, &signal, price)
                    .await;
            }

            // Save account snapshot
            match self.state.broker.get_account().await {
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

    async fn process_signal(
        &mut self,
        account_id: &str,
        strategy_name: &str,
        signal: &Signal,
        current_price: f64,
    ) {
        match signal {
            Signal::Buy { symbol, strength } => {
                self.state.logger.info(&format!(
                    "[{}][{}] BUY {} (strength: {:.2})",
                    account_id, strategy_name, symbol, strength
                ));
            }
            Signal::Sell { symbol, strength } => {
                self.state.logger.info(&format!(
                    "[{}][{}] SELL {} (strength: {:.2})",
                    account_id, strategy_name, symbol, strength
                ));
            }
            Signal::Hold => return,
        }

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

        let account = match self.state.broker.get_account().await {
            Ok(a) => a,
            Err(e) => {
                self.state
                    .logger
                    .error(&format!("[{}] Failed to fetch account: {}", account_id, e));
                return;
            }
        };

        let positions = match self.state.broker.get_positions().await {
            Ok(p) => p,
            Err(e) => {
                self.state.logger.error(&format!(
                    "[{}] Failed to fetch positions: {}",
                    account_id, e
                ));
                return;
            }
        };

        if let Some(order) =
            self.risk_manager
                .evaluate_signal(signal, &account, &positions, current_price)
        {
            self.state.logger.info(&format!(
                "[{}] Executing: {:?} {} x {}",
                account_id, order.side, order.symbol, order.quantity
            ));

            match self.state.broker.submit_order(&order).await {
                Ok(order_id) => {
                    self.state
                        .logger
                        .info(&format!("[{}] Order filled: {}", account_id, order_id));

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
                    self.state
                        .logger
                        .error(&format!("[{}] Order failed: {}", account_id, e));
                }
            }
        }
    }

    async fn fetch_historical_data(&mut self, symbols: &[String]) {
        self.state
            .logger
            .info("Fetching historical data for all symbols...");

        let mut by_config: HashMap<(String, u32), Vec<String>> = HashMap::new();

        for strategies in self.accounts.values() {
            for strategy in strategies {
                let (timeframe, limit) = strategy
                    .startup_config()
                    .unwrap_or(("1Hour".to_string(), 50));

                for symbol in strategy.symbols() {
                    if symbols.contains(symbol) {
                        let entry = by_config.entry((timeframe.clone(), limit)).or_default();
                        if !entry.contains(symbol) {
                            entry.push(symbol.clone());
                        }
                    }
                }
            }
        }

        for ((timeframe, limit), group_symbols) in by_config {
            match self
                .state
                .data
                .get_bars_batch(&group_symbols, &timeframe, limit)
                .await
            {
                Ok(bars_map) => {
                    for (symbol, bars) in &bars_map {
                        self.state.logger.info(&format!(
                            "{}: loaded {} bars ({})",
                            symbol,
                            bars.len(),
                            timeframe
                        ));

                        for strategies in self.accounts.values_mut() {
                            for strategy in strategies.iter_mut() {
                                if strategy.symbols().contains(symbol) {
                                    for bar in bars {
                                        let _ = strategy.on_bar(bar);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    self.state
                        .logger
                        .error(&format!("Failed to load history: {}", e));
                }
            }
        }

        self.state.logger.info("Historical data warmup complete");
    }
}
