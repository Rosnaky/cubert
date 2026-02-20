
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct DbTrade {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: String,
    pub order_id: Option<String>,
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbPosition {
    pub id: String,
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbAccountSnapshot {
    pub id: String,
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbSignal {
    pub id: String,
    pub symbol: String,
    pub signal_type: String,
    pub strength: f64,
    pub strategy: String,
    pub timestamp: String,
}
