use std::collections::HashMap;

use crate::{broker::{Broker, BrokerError, OrderId}, types::{Account, Order, OrderType, Position, Side}};


pub struct PaperBroker {
    account: Account,
    positions: HashMap<String, Position>,
    next_order_id: u64,
    prices: HashMap<String, f64>
}

impl PaperBroker {
    pub fn new(starting_cash: f64) -> Self {
        Self {
            account: Account { equity: starting_cash, cash: starting_cash, buying_power: starting_cash },
            positions: HashMap::new(),
            next_order_id: 1,
            prices: HashMap::new()
        }
    }

    pub fn set_price(&mut self, symbol: &str, price: f64) {
        self.prices.insert(symbol.to_string(), price);
        if let Some(pos) = self.positions.get_mut(symbol) {
            pos.current_price = price;
        }
    }

    fn get_price(&mut self, symbol: &str) -> Option<f64> {
        self.prices.get(symbol).copied()
    }

    fn generate_order_id(&mut self) -> OrderId {
        let id = format!("PAPER-{:06}", self.next_order_id);
        self.next_order_id += 1;
        id
    }

    fn update_account(&mut self) {
        let positions_value: f64 = self.positions.values()
        .map(|p| p.quantity * p.current_price)
        .sum();

        self.account.equity = positions_value + self.account.cash;
        self.account.buying_power = self.account.cash;
    }
}

impl Broker for PaperBroker {
    fn submit_order(&mut self, order: &Order) -> Result<OrderId, BrokerError> {
        let price = match order.order_type {
            OrderType::Market => {
                self.get_price(&order.symbol)
                .ok_or_else(|| BrokerError::InvalidOrder(
                    format!("No price for {}", order.symbol)
                ))?
            }
            OrderType::Limit { price } => price,
            OrderType::Stop { price } => price,
        };

        let order_value = price * order.quantity;

        match order.side {
            Side::Buy => {
                if order_value > self.account.cash  {
                    return Err(BrokerError::InsufficientFunds);
                }

                self.account.cash -= order_value;

                if let Some(pos) = self.positions.get_mut(&order.symbol) {
                    let total_qty = pos.quantity + order.quantity;
                    let total_cost = (pos.quantity * pos.avg_entry_price) + order_value;
                    pos.avg_entry_price = total_cost / total_qty;
                    pos.quantity = total_qty;
                    pos.current_price = price;
                }
                else {
                    self.positions.insert(order.symbol.clone(), Position { 
                        symbol: order.symbol.clone(), quantity: order.quantity, avg_entry_price: price, current_price: price 
                    });
                }
            }
            Side::Sell => {
                let pos = self.positions.get_mut(&order.symbol)
                .ok_or_else(|| BrokerError::InvalidOrder(format!("No position for {}", order.symbol)))?;

                if order.quantity > pos.quantity {
                    return Err(BrokerError::InvalidOrder("Insufficient shares".to_string()));
                }

                self.account.cash += order_value;

                pos.quantity -= order.quantity;

                if pos.quantity <= 0.0 {
                    self.positions.remove(&order.symbol);
                }
            }
        }

        self.update_account();
        Ok(self.generate_order_id())
    }
    
    fn cancel_order(&mut self, _order_id: &OrderId) -> Result<(), BrokerError> {
        Ok(())
    }
    
    fn get_position(&self, symbol: &str) -> Option<Position> {
        self.positions.get(symbol).cloned()
    }
    
    fn get_positions(&self) -> Vec<Position> {
        self.positions.values().cloned().collect()
    }
    
    fn get_account(&self) -> Account {
        self.account.clone()
    }
}