use async_trait::async_trait;

use crate::types::{Account, Order, Position};

pub mod alpaca;
pub mod paper;
pub mod paper_alpaca;

pub type OrderId = String;

#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    OrderRejected(String),
    InsufficientFunds,
    InvalidOrder(String),
    Fail(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            BrokerError::OrderRejected(msg) => write!(f, "Order rejected: {}", msg),
            BrokerError::InsufficientFunds => write!(f, "Insufficient funds"),
            BrokerError::InvalidOrder(msg) => write!(f, "Invalid order: {}", msg),
            BrokerError::Fail(msg) => write!(f, "Unknown failure: {}", msg),
        }
    }
}

impl std::error::Error for BrokerError {}

#[async_trait]
pub trait Broker: Send + Sync {
    async fn get_account(&self) -> Result<Account, BrokerError>;
    async fn get_positions(&self) -> Result<Vec<Position>, BrokerError>;
    async fn get_position(&self, symbol: &str) -> Result<Option<Position>, BrokerError>;
    async fn submit_order(&self, order: &Order) -> Result<OrderId, BrokerError>; // No current_price
    async fn update_prices(&self, symbols: &[String]) -> Result<(), BrokerError>; // New
}
