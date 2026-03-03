use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    broker::{Broker, BrokerError, OrderId},
    types::{Account, Order, OrderType, Position, Side},
};

#[derive(Debug, Deserialize)]
struct AlpacaApiAccount {
    equity: String,
    cash: String,
    buying_power: String,
}

impl AlpacaApiAccount {
    fn into_account(self) -> Account {
        Account {
            equity: self.equity.parse().unwrap_or(0.0),
            cash: self.cash.parse().unwrap_or(0.0),
            buying_power: self.buying_power.parse().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AlpacaApiPosition {
    symbol: String,
    qty: String,
    avg_entry_price: String,
    current_price: String,
}

impl AlpacaApiPosition {
    fn into_position(self) -> Position {
        Position {
            symbol: self.symbol,
            quantity: self.qty.parse().unwrap_or(0.0),
            avg_entry_price: self.avg_entry_price.parse().unwrap_or(0.0),
            current_price: self.current_price.parse().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AlpacaApiOrderResponse {
    id: String,
    _status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AlpacaApiOrderRequest {
    symbol: String,
    qty: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    time_in_force: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_price: Option<String>,
}

impl AlpacaApiOrderRequest {
    fn from_order(order: &Order) -> Self {
        let (order_type, limit_price, stop_price) = match order.order_type {
            OrderType::Market => ("market", None, None),
            OrderType::Limit { price } => ("limit", Some(price.to_string()), None),
            OrderType::Stop { price } => ("stop", None, Some(price.to_string())),
        };

        Self {
            symbol: order.symbol.clone(),
            qty: order.quantity.to_string(),
            side: match order.side {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
            .to_string(),
            order_type: order_type.to_string(),
            time_in_force: "day".to_string(),
            limit_price,
            stop_price,
        }
    }
}

pub struct AlpacaApiBroker {
    client: Client,
    base_url: String,
    api_key: String,
    api_secret: String,
}

impl AlpacaApiBroker {
    pub fn new(api_endpoint: &str, api_key: &str, api_secret: &str, _paper: bool) -> Self {
        Self {
            client: Client::new(),
            base_url: { api_endpoint }.to_string(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
        }
    }

    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", self.api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", self.api_secret.parse().unwrap());
        headers
    }

    pub async fn fetch_account(&self) -> Result<Account, BrokerError> {
        let resp = self
            .client
            .get(format!("{}/v2/account", self.base_url))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| BrokerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(BrokerError::Fail(resp.text().await.unwrap_or_default()));
        }

        let api_account = resp
            .json::<AlpacaApiAccount>()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        Ok(api_account.into_account())
    }

    pub async fn fetch_positions(&self) -> Result<Vec<Position>, BrokerError> {
        let resp = self
            .client
            .get(format!("{}/v2/positions", self.base_url))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(BrokerError::Fail(resp.text().await.unwrap_or_default()));
        }

        let api_positions = resp
            .json::<Vec<AlpacaApiPosition>>()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        Ok(api_positions
            .into_iter()
            .map(|p| p.into_position())
            .collect())
    }

    pub async fn submit_order_internal(&self, order: &Order) -> Result<OrderId, BrokerError> {
        let req = AlpacaApiOrderRequest::from_order(order);

        let resp = self
            .client
            .post(format!("{}/v2/orders", self.base_url))
            .headers(self.auth_headers())
            .json(&req)
            .send()
            .await
            .map_err(|e| BrokerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(BrokerError::OrderRejected(
                resp.text().await.unwrap_or_default(),
            ));
        }

        let order_resp = resp
            .json::<AlpacaApiOrderResponse>()
            .await
            .map_err(|e| BrokerError::Fail(e.to_string()))?;

        Ok(order_resp.id)
    }

    pub async fn cancel_order(&self, order_id: &OrderId) -> Result<(), BrokerError> {
        let resp = self
            .client
            .delete(format!("{}/v2/orders/{}", self.base_url, order_id))
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| BrokerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(BrokerError::Fail(resp.text().await.unwrap_or_default()));
        }

        Ok(())
    }
}

#[async_trait]
impl Broker for AlpacaApiBroker {
    async fn get_account(&self) -> Result<Account, BrokerError> {
        self.fetch_account().await
    }

    async fn get_positions(&self) -> Result<Vec<Position>, BrokerError> {
        self.fetch_positions().await
    }

    async fn get_position(&self, symbol: &str) -> Result<Option<Position>, BrokerError> {
        let positions = self.fetch_positions().await?;
        Ok(positions.into_iter().find(|p| p.symbol == symbol))
    }

    async fn submit_order(&self, order: &Order) -> Result<OrderId, BrokerError> {
        self.submit_order_internal(order).await // Real broker handles price
    }

    async fn update_prices(&self, _symbols: &[String]) -> Result<(), BrokerError> {
        Ok(()) // No-op for real broker - has real-time prices
    }
}
