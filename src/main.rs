use std::sync::Arc;

use cubert::config::Config;
use cubert::logging::Logger;
use cubert::broker::alpaca::AlpacaApiBroker;
use cubert::data::MarketData;
use cubert::storage::Storage;
use cubert::engine::Engine;
use cubert::strategy::{StrategyParams, StrategySettings};

#[tokio::main]
async fn main() {
    // Load config
    let config = match Config::load("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Create logger
    let level = Logger::parse_level(&config.logging.level);
    let logger = Arc::new(
        Logger::new(level, Some(&config.logging.file))
            .expect("Failed to create logger")
    );

    logger.info("=== Cubert Starting ===");

    // Connect to database
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

    // Create broker and data clients
    let broker = AlpacaApiBroker::new(
        &config.broker.api_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
        config.broker.paper,
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

    let data = MarketData::new(
        &config.broker.data_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
    );

    let mut engine = Engine::new(
        storage,
        broker,
        data,
        logger.clone(),
        config.risk.clone(),
        strategies.clone(),
    ).await;

    for strategy in strategies {
        engine.add_strategy(strategy);
    }

    // Run engine (polls every 60 seconds)
    engine.run(60).await;
}