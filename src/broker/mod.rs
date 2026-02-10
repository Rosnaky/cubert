use crate::types::{Account, Order, Position};


pub mod paper;

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

pub trait Broker {
    fn submit_order(&mut self, order: &Order) -> Result<OrderId, BrokerError>;
    fn cancel_order(&mut self, order_id: &OrderId) -> Result<(), BrokerError>;
    fn get_position(&self, symbol: &str) -> Option<Position>;
    fn get_positions(&self) -> Vec<Position>;
    fn get_account(&self) -> Account;
}
