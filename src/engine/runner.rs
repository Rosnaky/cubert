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

pub struct SharedState {
    pub storage: Storage,
    pub data: MarketData,
    pub logger: Arc<Logger>,
}

pub struct AccountRunner {
    pub broker: Arc<dyn Broker>,
    pub strategies: Vec<Box<dyn Strategy>>,
}

pub struct Engine {
    state: Arc<SharedState>,
    accounts: HashMap<String, AccountRunner>,
    risk_manager: RiskManager,
    _signal_tx: mpsc::Sender<SignalMessage>,
    _signal_rx: mpsc::Receiver<SignalMessage>,
}

impl Engine {
    pub async fn new(
        storage: Storage,
        data: MarketData,
        logger: Arc<Logger>,
        risk_config: RiskConfig,
    ) -> Self {
        let (signal_tx, signal_rx) = mpsc::channel(100);

        let state = Arc::new(SharedState {
            storage,
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

    pub async fn load_strategies(
        &mut self,
        account_id: &str,
        broker: Arc<dyn Broker>,
    ) -> Result<(), String> {
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
                "[{}] Loaded strategy: {} for {:?}",
                account_id,
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
        self.accounts
            .insert(account_id.to_string(), AccountRunner { broker, strategies });
        Ok(())
    }

    fn all_symbols(&self) -> Vec<String> {
        let mut symbols = Vec::new();
        for runner in self.accounts.values() {
            for strategy in &runner.strategies {
                for symbol in strategy.symbols() {
                    if !symbols.contains(symbol) {
                        symbols.push(symbol.clone());
                    }
                }
            }
        }
        symbols
    }

    pub async fn run(&mut self, poll_interval_secs: u64) {
        self.state.logger.info("Engine starting...");

        let all_symbols = self.all_symbols();
        self.state.logger.info(&format!(
            "Tracking {} symbols across {} accounts: {:?}",
            all_symbols.len(),
            self.accounts.len(),
            all_symbols
        ));

        self.fetch_historical_data(&all_symbols).await;

        let mut ticker = interval(Duration::from_secs(poll_interval_secs));

        loop {
            self.state.logger.info("=== Tick starting ===");

            let all_symbols = self.all_symbols();

            for runner in self.accounts.values() {
                runner.broker.update_prices(&all_symbols).await.ok();
            }

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

            let mut all_signals: Vec<(String, String, Signal, f64)> = Vec::new();

            for (account_id, runner) in &mut self.accounts {
                for strategy in runner.strategies.iter_mut() {
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

            for (account_id, strategy_name, signal, price) in all_signals {
                self.process_signal(&account_id, &strategy_name, &signal, price)
                    .await;
            }

            for (account_id, runner) in &self.accounts {
                match runner.broker.get_account().await {
                    Ok(account) => {
                        let _ = self
                            .state
                            .storage
                            .insert_account_snapshot(
                                account_id,
                                account.equity,
                                account.cash,
                                account.buying_power,
                            )
                            .await;
                        self.state
                            .logger
                            .info(&format!("[{}] Account: ${:.2}", account_id, account.equity));
                    }
                    Err(e) => {
                        self.state
                            .logger
                            .error(&format!("[{}] Failed to fetch account: {}", account_id, e));
                    }
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
        let runner = match self.accounts.get(account_id) {
            Some(r) => r,
            None => return,
        };

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
            .insert_signal(account_id, &symbol, signal_type, strength, strategy_name)
            .await;

        let account = match runner.broker.get_account().await {
            Ok(a) => a,
            Err(e) => {
                self.state
                    .logger
                    .error(&format!("[{}] Failed to fetch account: {}", account_id, e));
                return;
            }
        };

        let positions = match runner.broker.get_positions().await {
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

            match runner.broker.submit_order(&order).await {
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
                            account_id,
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

        for runner in self.accounts.values() {
            for strategy in &runner.strategies {
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

                        for runner in self.accounts.values_mut() {
                            for strategy in runner.strategies.iter_mut() {
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
