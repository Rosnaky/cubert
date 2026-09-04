use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

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
    file: Option<Mutex<File>>,
    strategy_dir: Option<PathBuf>,
    strategy_files: Mutex<HashMap<String, File>>,
}

impl Logger {
    pub fn new(
        level: Level,
        file_path: Option<&str>,
        strategy_dir: Option<&str>,
    ) -> std::io::Result<Self> {
        let file = if let Some(path) = file_path {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let f = OpenOptions::new().create(true).append(true).open(path)?;
            Some(Mutex::new(f))
        } else {
            None
        };

        let strategy_dir = strategy_dir.map(PathBuf::from).or_else(|| {
            file_path.and_then(|p| Path::new(p).parent().map(|d| d.join("strategies")))
        });

        if let Some(ref dir) = strategy_dir {
            std::fs::create_dir_all(dir)?;
        }

        Ok(Logger {
            level,
            file,
            strategy_dir,
            strategy_files: Mutex::new(HashMap::new()),
        })
    }

    fn sanitize(value: &str) -> String {
        value
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn file_stem(id: &str, name: &str) -> String {
        let name = Self::sanitize(name);
        let id = Self::sanitize(id);

        match (name.is_empty(), id.is_empty()) {
            (true, true) => "unnamed".to_string(),
            (true, false) => id,
            (false, true) => name,
            (false, false) => format!("{}_{}", name, id),
        }
    }

    fn write_strategy_line(&self, id: &str, name: &str, line: &str) {
        let dir = match self.strategy_dir {
            Some(ref d) => d,
            None => return,
        };

        let mut files = match self.strategy_files.lock() {
            Ok(f) => f,
            Err(_) => return,
        };

        if !files.contains_key(id) {
            let path = dir.join(format!("{}.log", Self::file_stem(id, name)));
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(f) => {
                    files.insert(id.to_string(), f);
                }
                Err(_) => return,
            }
        }

        if let Some(f) = files.get_mut(id) {
            let _ = writeln!(f, "{}", line);
        }
    }

    fn log_strategy(&self, id: &str, name: &str, level: Level, message: &str) {
        if level < self.level {
            return;
        }

        self.log(level, message);

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        self.write_strategy_line(
            id,
            name,
            &format!("[{}] [{}] {}", timestamp, level, message),
        );
    }

    pub fn strategy_debug(&self, id: &str, name: &str, message: &str) {
        self.log_strategy(id, name, Level::Debug, message);
    }

    pub fn strategy_info(&self, id: &str, name: &str, message: &str) {
        self.log_strategy(id, name, Level::Info, message);
    }

    pub fn strategy_error(&self, id: &str, name: &str, message: &str) {
        self.log_strategy(id, name, Level::Error, message);
    }

    pub fn parse_level(s: &str) -> Level {
        match s.to_lowercase().as_str() {
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            _ => Level::Info,
        }
    }

    fn log(&self, level: Level, message: &str) {
        if level < self.level {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{}] {}", timestamp, level, message);

        println!("{}", line);

        if let Some(ref file_mutex) = self.file
            && let Ok(mut file) = file_mutex.lock()
        {
            let _ = writeln!(file, "{}", line);
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
