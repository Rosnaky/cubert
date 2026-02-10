use cubert::config::Config;
use cubert::logging::Logger;
use cubert::types::{Signal, Order, Side, OrderType, Position};

fn main() {
    // Load config
    let config = match Config::load("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Create logger from config
    let level = Logger::parse_level(&config.logging.level);
    let logger = Logger::new(level, Some(&config.logging.file))
        .expect("Failed to create logger");

    logger.info("Quant bot starting up...");
    logger.debug("This is a debug message");
    logger.info(&format!("Trading symbols: {:?}", config.data.symbols));
    logger.warn("This is a warning");

    // Log a signal
    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.75,
    };
    logger.log_signal(&signal);

    // Log an order
    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 10.0,
        order_type: OrderType::Market,
    };
    logger.log_order(&order);

    // Log a position
    let position = Position {
        symbol: "AAPL".to_string(),
        quantity: 100.0,
        avg_entry_price: 150.0,
        current_price: 157.50,
    };
    logger.log_position(&position);

    logger.info("Startup complete");
}