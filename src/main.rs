use std::sync::Arc;

use cubert::broker::Broker;
use cubert::broker::alpaca::AlpacaApiBroker;
use cubert::broker::paper_alpaca::PaperAlpacaBroker;
use cubert::config::Config;
use cubert::data::MarketData;
use cubert::engine::Engine;
use cubert::logging::Logger;
use cubert::storage::Storage;

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

    // Engine storage (for logging signals, snapshots, etc.)
    let storage = match Storage::connect(&config.storage.db_url).await {
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

    // Create broker based on config
    let broker: Arc<dyn Broker> = if config.broker.paper {
        logger.info("Using paper broker (simulated)");

        // Broker storage (simulates broker API)
        let broker_storage = match Storage::connect(&config.storage.broker_db_url).await {
            Ok(s) => s,
            Err(e) => {
                logger.error(&format!("Broker database error: {}", e));
                std::process::exit(1);
            }
        };

        if let Err(e) = broker_storage.migrate().await {
            logger.error(&format!("Broker migration failed: {}", e));
            std::process::exit(1);
        }

        let starting_cash = 100_000.0;
        if let Err(e) = broker_storage.init_account(starting_cash).await {
            logger.error(&format!("Failed to init account: {}", e));
            std::process::exit(1);
        }

        let data = MarketData::new(
            &config.broker.data_endpoint,
            &config.broker.api_key,
            &config.broker.api_secret,
        );

        Arc::new(PaperAlpacaBroker::new(broker_storage, data))
    } else {
        logger.info("Using Alpaca live broker");

        Arc::new(AlpacaApiBroker::new(
            &config.broker.api_endpoint,
            &config.broker.api_key,
            &config.broker.api_secret,
            false,
        ))
    };

    // Verify account
    match broker.get_account().await {
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

    // Market data
    let data = MarketData::new(
        &config.broker.data_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
    );

    let mut engine = Engine::new(storage, broker, data, logger.clone(), config.risk.clone()).await;

    if let Err(e) = engine.load_all_accounts().await {
        logger.error(&format!("Failed to load accounts: {}", e));
        std::process::exit(1);
    }

    engine.run(60).await;
}
