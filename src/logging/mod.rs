use std::{fs::{File, OpenOptions}, path::Path, sync::Mutex, io::Write};

use chrono::Local;

use crate::types::{Order, Position, Signal};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Debug => write!(f, "DEBUG"),
            Level::Info => write!(f, "INFO"),
            Level::Warn => write!(f, "WARN"),
            Level::Error => write!(f, "ERROR"),
        }
    }
}

pub struct Logger {
    level: Level,
    file: Option<Mutex<File>>
}

impl Logger {
    pub fn new(level: Level, file_path: Option<&str>) -> std::io::Result<Self> {
        let file = if let Some(path) = file_path {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            Some(Mutex::new(f))
        } 
        else {
            None
        };

        Ok(Logger { level, file })
    }

    pub fn parse_level(s: &str) -> Level {
        match s.to_lowercase().as_str() {
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            _ => Level::Info
        }
    }

    fn log(&self, level: Level, message: &str) {
        if level < self.level {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{}] {}", timestamp, level, message);

        println!("{}", line);

        if let Some(ref file_mutex) = self.file {
            if let Ok(mut file) = file_mutex.lock() {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    pub fn debug(&self, message: &str) {
        self.log(Level::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(Level::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(Level::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(Level::Error, message);
    }

    pub fn log_signal(&self, signal: &Signal) {
        let msg = match signal {
            Signal::Buy { symbol, strength } => {
                format!("SIGNAL: BUY {} (strength: {:.2})", symbol, strength)
            }
            Signal::Sell { symbol, strength } => {
                format!("SIGNAL: SELL {} (strength: {:.2})", symbol, strength)
            }
            Signal::Hold => "SIGNAL: HOLD".to_string(),
        };
        self.log(Level::Info, &msg);
    }

    pub fn log_order(&self, order: &Order) {
        let msg = format!(
            "ORDER: {:?} {} x {:.2} @ {:?}",
            order.side, order.symbol, order.quantity, order.order_type
        );
        self.log(Level::Info, &msg);
    }

    pub fn log_position(&self, position: &Position) {
        let msg = format!(
            "POSITION: {} x {:.2} @ {:.2} (P&L: ${:.2}, {:.2}%)",
            position.symbol,
            position.quantity,
            position.avg_entry_price,
            position.unrealized_pnl(),
            position.unrealized_pnl_pct()
        );
        self.log(Level::Info, &msg);
    }
}