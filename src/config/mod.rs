use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub broker: BrokerConfig,
    pub data: DataConfig,
    pub risk: RiskConfig,
    pub logging: LoggingConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
pub struct BrokerConfig {
    pub name: String,
    pub paper: bool,
    pub api_endpoint: String,
    pub data_endpoint: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct DataConfig {
    pub symbols: Vec<String>,
    pub timeframe: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StrategyConfig {
    // Renamed from StrategyConfig
    pub name: String,
    pub symbols: Vec<String>,
    pub params: StrategyParamsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StrategyParamsConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub lookback_period: usize,
    pub threshold: f64,
    pub startup_lookback: String,
    pub startup_bar_limit: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RiskConfig {
    pub max_position_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_daily_trades: u32,
    #[serde(default = "default_max_total_exposure")]
    pub max_total_exposure: f64,
    #[serde(default = "default_max_loss_per_trade")]
    pub max_loss_per_trade: f64,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub db_url: String,
    pub broker_db_url: String,
}

#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(String),
    IoError(std::io::Error),
    MissingEnvVar(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse config: {}", msg),
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::MissingEnvVar(var) => write!(f, "Missing environment variable: {}", var),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        dotenv::dotenv().ok();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let contents = std::fs::read_to_string(path)?;

        let mut config: Config =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.broker.api_key = std::env::var("ALPACA_API_KEY")
            .map_err(|_| ConfigError::MissingEnvVar("ALPACA_API_KEY".to_string()))?;

        config.broker.api_secret = std::env::var("ALPACA_API_SECRET")
            .map_err(|_| ConfigError::MissingEnvVar("ALPACA_API_SECRET".to_string()))?;

        Ok(config)
    }
}

fn default_max_total_exposure() -> f64 {
    0.8
}

fn default_max_loss_per_trade() -> f64 {
    0.02
}
