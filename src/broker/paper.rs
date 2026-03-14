use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{
    broker::{Broker, BrokerError, OrderId},
    types::{Account, Order, OrderType, Position, Side},
};

pub struct PaperBroker {
    account: Mutex<Account>,
    positions: Mutex<HashMap<String, Position>>,
    next_order_id: Mutex<u64>,
    prices: Mutex<HashMap<String, f64>>,
}

impl PaperBroker {
    pub fn new(id: &str, starting_cash: f64) -> Self {
        Self {
            account: Mutex::new(Account {
                id: id.to_string(),
                equity: starting_cash,
                cash: starting_cash,
                buying_power: starting_cash,
            }),
            positions: Mutex::new(HashMap::new()),
            next_order_id: Mutex::new(1),
            prices: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_price(&self, symbol: &str, price: f64) {
        self.prices
            .lock()
            .unwrap()
            .insert(symbol.to_string(), price);
        if let Some(pos) = self.positions.lock().unwrap().get_mut(symbol) {
            pos.current_price = price;
        }
    }

    fn get_price(&self, symbol: &str) -> Option<f64> {
        self.prices.lock().unwrap().get(symbol).copied()
    }

    fn generate_order_id(&self) -> OrderId {
        let mut id_counter = self.next_order_id.lock().unwrap();
        let id = format!("PAPER-{:06}", *id_counter);
        *id_counter += 1;
        id
    }

    fn update_account(&self) {
        let positions = self.positions.lock().unwrap();
        let positions_value: f64 = positions
            .values()
            .map(|p| p.quantity * p.current_price)
            .sum();

        let mut account = self.account.lock().unwrap();
        account.equity = positions_value + account.cash;
        account.buying_power = account.cash;
    }
}

#[async_trait]
impl Broker for PaperBroker {
    async fn get_account(&self) -> Result<Account, BrokerError> {
        Ok(self.account.lock().unwrap().clone())
    }

    async fn get_positions(&self) -> Result<Vec<Position>, BrokerError> {
        Ok(self.positions.lock().unwrap().values().cloned().collect())
    }

    async fn get_position(&self, symbol: &str) -> Result<Option<Position>, BrokerError> {
        Ok(self.positions.lock().unwrap().get(symbol).cloned())
    }

    async fn submit_order(&self, order: &Order) -> Result<OrderId, BrokerError> {
        let price = match order.order_type {
            OrderType::Market => self.get_price(&order.symbol).ok_or_else(|| {
                BrokerError::InvalidOrder(format!("No price for {}", order.symbol))
            })?,
            OrderType::Limit { price } => price,
            OrderType::Stop { price } => price,
        };

        let order_value = price * order.quantity;

        match order.side {
            Side::Buy => {
                {
                    let account = self.account.lock().unwrap();
                    if order_value > account.cash {
                        return Err(BrokerError::InsufficientFunds);
                    }
                }

                self.account.lock().unwrap().cash -= order_value;

                let mut positions = self.positions.lock().unwrap();
                if let Some(pos) = positions.get_mut(&order.symbol) {
                    let total_qty = pos.quantity + order.quantity;
                    let total_cost = (pos.quantity * pos.avg_entry_price) + order_value;
                    pos.avg_entry_price = total_cost / total_qty;
                    pos.quantity = total_qty;
                    pos.current_price = price;
                } else {
                    positions.insert(
                        order.symbol.clone(),
                        Position {
                            symbol: order.symbol.clone(),
                            quantity: order.quantity,
                            avg_entry_price: price,
                            current_price: price,
                        },
                    );
                }
            }
            Side::Sell => {
                let mut positions = self.positions.lock().unwrap();
                let pos = positions.get_mut(&order.symbol).ok_or_else(|| {
                    BrokerError::InvalidOrder(format!("No position for {}", order.symbol))
                })?;

                if order.quantity > pos.quantity {
                    return Err(BrokerError::InvalidOrder("Insufficient shares".to_string()));
                }

                self.account.lock().unwrap().cash += order_value;

                pos.quantity -= order.quantity;

                if pos.quantity <= 0.0 {
                    let symbol = order.symbol.clone();
                    drop(positions);
                    self.positions.lock().unwrap().remove(&symbol);
                }
            }
        }

        self.update_account();
        Ok(self.generate_order_id())
    }

    async fn update_prices(&self, symbols: &[String]) -> Result<(), BrokerError> {
        // PaperBroker uses set_price() externally, so this is a no-op
        // or you could update positions with stored prices
        let prices = self.prices.lock().unwrap();
        let mut positions = self.positions.lock().unwrap();

        for symbol in symbols {
            if let Some(price) = prices.get(symbol)
                && let Some(pos) = positions.get_mut(symbol)
            {
                pos.current_price = *price;
            }
        }

        drop(positions);
        drop(prices);
        self.update_account();

        Ok(())
    }
}
