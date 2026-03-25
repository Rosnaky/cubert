use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::strategy::StrategyParams;

#[derive(Debug, Clone, FromRow)]
pub struct DbTrade {
    pub id: String,
    pub account_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: String,
    pub order_id: Option<String>,
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbSignal {
    pub id: String,
    pub account_id: String,
    pub symbol: String,
    pub signal_type: String,
    pub strength: f64,
    pub strategy: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbAccountSnapshot {
    pub id: String,
    pub account_id: String,
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbPosition {
    pub id: String,
    pub account_id: String,
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbAccount {
    pub id: String,
    pub cash: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbStrategy {
    pub id: String,
    pub name: String,
    pub strategy_type: String,
    pub params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl DbStrategy {
    pub fn new(name: String, params: &StrategyParams) -> Self {
        let now = Utc::now().to_rfc3339();
        let strategy_type = match params {
            StrategyParams::Momentum { .. } => "momentum",
            StrategyParams::MeanReversion { .. } => "mean_reversion",
            _ => "custom",
        };

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            strategy_type: strategy_type.to_string(),
            params_json: serde_json::to_string(params).unwrap_or_default(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn params(&self) -> Result<StrategyParams, serde_json::Error> {
        serde_json::from_str(&self.params_json)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct DbAccountStrategy {
    pub account_id: String,
    pub strategy_id: String,
    pub symbols_json: String,
    pub enabled: bool,
    pub assigned_at: String,
}
