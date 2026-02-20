

use crate::config::RiskConfig;
use crate::types::OrderType;
use crate::types::Side;
use crate::types::Order;
use crate::types::Position;
use crate::types::Signal;

use crate::types::Account;

pub struct RiskManager {
    config: RiskConfig,
    daily_trades: u32,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            daily_trades: 0,
        }
    }

    pub fn evaluate_signal(
        &mut self,
        signal: &Signal,
        account: &Account,
        positions: &[Position],
        current_price: f64,
    ) -> Option<Order> {
        match signal {
            Signal::Hold => None,

            Signal::Buy { symbol, strength } => {
                if self.daily_trades >= self.config.max_daily_trades {
                    return None;
                }

                let existing = positions.iter().find(|p| &p.symbol == symbol);
                if existing.is_some() {
                    return None;
                }

                let max_value = account.equity * self.config.max_position_pct;
                let target_value = max_value * strength;
                let quantity = (target_value / current_price).floor();

                if quantity < 1.0 {
                    return None;
                }

                let current_exposure: f64 = positions.iter()
                    .map(|p| p.quantity * p.current_price)
                    .sum();

                let new_exposure = current_exposure + (quantity * current_price);

                if new_exposure > account.equity * self.config.max_total_exposure {
                    return None;
                }

                self.daily_trades += 1;

                Some(Order {
                    symbol: symbol.clone(),
                    side: Side::Buy,
                    quantity,
                    order_type: OrderType::Market,
                })
            }

            Signal::Sell { symbol, strength: _ } => {
                let position = positions.iter().find(|p| &p.symbol == symbol)?;

                if position.quantity <= 0.0 {
                    return None;
                }

                self.daily_trades += 1;
                
                Some(Order {
                    symbol: symbol.clone(),
                    side: Side::Sell,
                    quantity: position.quantity,
                    order_type: OrderType::Market,
                })
            }
        }
    }

    pub fn reset_daily(&mut self) {
        self.daily_trades = 0;
    }
}
