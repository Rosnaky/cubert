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
    let logger = Arc::new(
        Logger::new(
            level,
            Some(&config.logging.file),
            config.logging.strategy_dir.as_deref(),
        )
        .expect("Failed to create logger"),
    );

    logger.info("=== Cubert Starting ===");

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

    let data = MarketData::new(
        &config.broker.data_endpoint,
        &config.broker.api_key,
        &config.broker.api_secret,
    );

    let account_ids = match storage.get_active_account_ids().await {
        Ok(ids) => ids,
        Err(e) => {
            logger.error(&format!("Failed to load accounts: {}", e));
            std::process::exit(1);
        }
    };

    logger.info(&format!("Found {} active accounts", account_ids.len()));
    let mut engine = Engine::new(storage, data.clone(), logger.clone(), config.risk.clone()).await;

    for account_id in &account_ids {
        let broker: Arc<dyn Broker> = if config.broker.paper {
            let broker_db_url = format!("sqlite:broker_{}.db", account_id);
            let broker_storage = match Storage::connect(&broker_db_url).await {
                Ok(s) => s,
                Err(e) => {
                    logger.error(&format!("[{}] Broker DB error: {}", account_id, e));
                    continue;
                }
            };

            if let Err(e) = broker_storage.migrate().await {
                logger.error(&format!("[{}] Broker migration failed: {}", account_id, e));
                continue;
            }

            if let Err(e) = broker_storage.init_account(account_id, 100_000.0).await {
                logger.error(&format!("[{}] Failed to init account: {}", account_id, e));
                continue;
            }

            Arc::new(PaperAlpacaBroker::new(
                account_id,
                broker_storage,
                data.clone(),
            ))
        } else {
            Arc::new(AlpacaApiBroker::new(
                account_id,
                &config.broker.api_endpoint,
                &config.broker.api_key,
                &config.broker.api_secret,
                false,
            ))
        };

        match broker.get_account().await {
            Ok(account) => {
                logger.info(&format!(
                    "[{}] Account: ${:.2} cash, ${:.2} equity",
                    account_id, account.cash, account.equity
                ));
            }
            Err(e) => {
                logger.error(&format!("[{}] Account error: {}", account_id, e));
                continue;
            }
        }

        if let Err(e) = engine.load_strategies(account_id, broker).await {
            logger.error(&format!(
                "[{}] Failed to load strategies: {}",
                account_id, e
            ));
        }
    }

    engine.run(60).await;
}
