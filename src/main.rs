use cubert::config::Config;
use cubert::logging::Logger;
use cubert::broker::alpaca::AlpacaApiBroker;

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
    let logger = Logger::new(level, Some(&config.logging.file))
        .expect("Failed to create logger");

    logger.info("=== Cubert Starting ===");

    let broker = AlpacaApiBroker::new(
        &config.broker.api_key,
        &config.broker.api_secret,
        config.broker.paper,
    );

    // Test account fetch
    match broker.fetch_account().await {
        Ok(account) => {
            logger.info(&format!("Account equity: ${:.2}", account.equity));
            logger.info(&format!("Cash: ${:.2}", account.cash));
            logger.info(&format!("Buying power: ${:.2}", account.buying_power));
        }
        Err(e) => {
            logger.error(&format!("Failed to fetch account: {}", e));
            std::process::exit(1);
        }
    }

    // Test positions fetch
    match broker.fetch_positions().await {
        Ok(positions) => {
            if positions.is_empty() {
                logger.info("No open positions");
            } else {
                for pos in &positions {
                    logger.log_position(pos);
                }
            }
        }
        Err(e) => {
            logger.error(&format!("Failed to fetch positions: {}", e));
        }
    }

    logger.info("=== Connection OK ===");
}