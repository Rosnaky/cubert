use std::sync::Arc;

use cubert::broker::alpaca::AlpacaApiBroker;
use cubert::config::Config;
use cubert::data::MarketData;
use cubert::engine::Engine;
use cubert::logging::Logger;
use cubert::storage::Storage;
use cubert::strategy::{StrategyParams, StrategySettings};

#[tokio::main]
async fn main() {
    let config = match Config::load("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let level = Logger::parse_level(&config.logging.level);
    let logger =
        Arc::new(Logger::new(level, Some(&config.logging.file)).expect("Failed to create logger"));

    logger.info("=== Cubert Starting ===");

    let storage = match Storage::connect(&config.storage.database_url).await {
        Ok(s) => s,
        Err(e) => {
            logger.error(&format!("Database error: {}", e));
            std::process::exit(1);
        }
    };

    if let Err(e) = storage.migrate().await {
        logger.error(&format!("Migration failed: {}", e));
        std::process::exit(1);
    }

    // Initialize account with starting cash (only creates if doesn't exist)
    let starting_cash = 100_000.0;
    if let Err(e) = storage.init_account(starting_cash).await {
        logger.error(&format!("Failed to init account: {}", e));
        std::process::exit(1);
    }

    // Verify account
    match storage.get_account().await {
        Ok(account) => {
            logger.info(&format!(
                "Account: ${:.2} cash, ${:.2} equity",
                account.cash, account.equity
            ));
        }
        Err(e) => {
            logger.error(&format!("Account error: {}", e));
            std::process::exit(1);
        }
    }

    let broker = AlpacaApiBroker::new(
        &config.broker.api_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
        config.broker.paper,
    );

    let data = MarketData::new(
        &config.broker.data_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
    );

    let strategies = vec![
        StrategySettings {
            name: "momentum_tech".to_string(),
            symbols: vec!["AAPL".to_string(), "MSFT".to_string()],
            params: StrategyParams::Momentum {
                lookback_period: 20,
                threshold: 0.02,
                startup_lookback: "1Hour".to_string(),
                startup_bar_limit: 50,
            },
        },
        StrategySettings {
            name: "momentum_ev".to_string(),
            symbols: vec!["TSLA".to_string()],
            params: StrategyParams::Momentum {
                lookback_period: 10,
                threshold: 0.03,
                startup_lookback: "30Min".to_string(),
                startup_bar_limit: 30,
            },
        },
    ];

    let mut engine = Engine::new(
        storage,
        broker,
        data,
        logger.clone(),
        config.risk.clone(),
        strategies.clone(),
    )
    .await;

    for strategy in strategies {
        engine.add_strategy(strategy);
    }

    engine.run(60).await;
}
