use crate::{
    broker::{Broker, BrokerError, OrderId},
    data::MarketData,
    storage::Storage,
    types::{Account, Order, Position, Side},
};
use async_trait::async_trait;

pub struct PaperAlpacaBroker {
    storage: Storage,
    data: MarketData,
}

impl PaperAlpacaBroker {
    pub fn new(storage: Storage, data: MarketData) -> Self {
        Self { storage, data }
    }
}

#[async_trait]
impl Broker for PaperAlpacaBroker {
    async fn get_account(&self) -> Result<Account, BrokerError> {
        self.storage
            .get_account()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))
    }

    async fn get_positions(&self) -> Result<Vec<Position>, BrokerError> {
        self.storage
            .get_positions()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))
    }

    async fn get_position(&self, symbol: &str) -> Result<Option<Position>, BrokerError> {
        self.storage
            .get_position(symbol)
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))
    }

    async fn submit_order(&self, order: &Order) -> Result<OrderId, BrokerError> {
        // Fetch current price
        let bar = self
            .data
            .get_latest_bar(&order.symbol)
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;
        let current_price = bar.close;

        let order_value = order.quantity * current_price;
        let order_id = uuid::Uuid::new_v4().to_string();

        match order.side {
            Side::Buy => {
                self.storage
                    .deduct_cash(order_value)
                    .await
                    .map_err(|_| BrokerError::InsufficientFunds)?;

                let existing = self
                    .storage
                    .get_position(&order.symbol)
                    .await
                    .map_err(|e| BrokerError::Fail(e.to_string()))?;

                if let Some(pos) = existing {
                    let total_qty = pos.quantity + order.quantity;
                    let total_cost = (pos.quantity * pos.avg_entry_price) + order_value;
                    let new_avg = total_cost / total_qty;
                    self.storage
                        .upsert_position(&order.symbol, total_qty, new_avg, current_price)
                        .await
                        .map_err(|e| BrokerError::Fail(e.to_string()))?;
                } else {
                    self.storage
                        .upsert_position(
                            &order.symbol,
                            order.quantity,
                            current_price,
                            current_price,
                        )
                        .await
                        .map_err(|e| BrokerError::Fail(e.to_string()))?;
                }
            }
            Side::Sell => {
                let existing = self
                    .storage
                    .get_position(&order.symbol)
                    .await
                    .map_err(|e| BrokerError::Fail(e.to_string()))?
                    .ok_or_else(|| BrokerError::InvalidOrder("No position".to_string()))?;

                if order.quantity > existing.quantity {
                    return Err(BrokerError::InvalidOrder("Insufficient shares".to_string()));
                }

                self.storage
                    .add_cash(order_value)
                    .await
                    .map_err(|e| BrokerError::Fail(e.to_string()))?;

                let new_qty = existing.quantity - order.quantity;
                if new_qty <= 0.0 {
                    self.storage
                        .delete_position(&order.symbol)
                        .await
                        .map_err(|e| BrokerError::Fail(e.to_string()))?;
                } else {
                    self.storage
                        .upsert_position(
                            &order.symbol,
                            new_qty,
                            existing.avg_entry_price,
                            current_price,
                        )
                        .await
                        .map_err(|e| BrokerError::Fail(e.to_string()))?;
                }
            }
        }

        let side_str = match order.side {
            Side::Buy => "buy",
            Side::Sell => "sell",
        };
        self.storage
            .insert_trade(
                &order.symbol,
                side_str,
                order.quantity,
                current_price,
                Some(&order_id),
                None, // No strategy name in broker - that's engine's concern
            )
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        Ok(order_id)
    }

    async fn update_prices(&self, symbols: &[String]) -> Result<(), BrokerError> {
        let bars = self
            .data
            .get_latest_bars(symbols)
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        for (symbol, bar) in &bars {
            self.storage
                .update_position_price(symbol, bar.close)
                .await
                .map_err(|e| BrokerError::Fail(e.to_string()))?;
        }

        let account = self
            .storage
            .get_account()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        self.storage
            .insert_account_snapshot(account.equity, account.cash, account.buying_power)
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        Ok(())
    }
}
