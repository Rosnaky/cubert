
//! Type defintions

use std::time::SystemTime;


#[derive(Debug, Clone)]
pub struct Bar {
    pub symbol: String,
    pub timestamp: SystemTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

#[derive(Debug, Clone)]
pub struct Tick {
    pub symbol: String,
    pub timestamp: SystemTime,
    pub price: f64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub enum Signal {
    // Strength is from 0 - 1.0
    Buy { symbol: String, strength: f64 },
    Sell { symbol: String, strength: f64 },
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderType {
    Market,
    Limit { price: f64 },
    Stop { price: f64 },
}

#[derive(Debug, Clone)]
pub struct Order {
    pub symbol: String,
    pub side: Side,
    pub quantity: f64,
    pub order_type: OrderType
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64, // Positive == long, negative == short
    pub avg_entry_price: f64,
    pub current_price: f64
}

impl Position {
    /// Get the unrealized Profit & Loss
    pub fn unrealized_pnl(&self) -> f64 {
        self.quantity * (self.current_price - self.avg_entry_price)
    }

    /// Get the unrealized Profit & Loss as a percentage
    pub fn unrealized_pnl_pct(&self) -> f64 {
        if self.avg_entry_price == 0.0 {
            return 0.0;
        }

        (self.current_price - self.avg_entry_price) / self.avg_entry_price * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
}

