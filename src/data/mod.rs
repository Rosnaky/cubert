use reqwest::Client;
use serde::Deserialize;

use crate::types::Bar;

#[derive(Debug)]
pub enum DataError {
    ConnectionFailed(String),
    ParseError(String),
    NotFound(String),
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            DataError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            DataError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for DataError {}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiBar {
    t: String, // Timestamp
    o: f64,    // Open
    h: f64,    // High
    l: f64,    // Low
    c: f64,    // Close
    v: u64,    // Volume
    #[serde(default)]
    n: Option<u64>, // Number of trades
    #[serde(default)]
    vw: Option<f64>, // Volume weighted price
}

// Single symbol response: { "bars": [...], "symbol": "AAPL" }
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiBarsResponse {
    bars: Option<Vec<ApiBar>>,
    symbol: String,
}

pub struct MarketData {
    client: Client,
    data_endpoint: String,
    api_key: String,
    api_secret: String,
}

impl MarketData {
    pub fn new(data_endpoint: &str, api_key: &str, api_secret: &str) -> Self {
        Self {
            client: Client::new(),
            data_endpoint: data_endpoint.to_string(),
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

    pub async fn get_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<Vec<Bar>, DataError> {
        let url = format!(
            "{}/v2/stocks/{}/bars?timeframe={}&limit={}",
            self.data_endpoint, symbol, timeframe, limit
        );

        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| DataError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DataError::ConnectionFailed(body));
        }

        let api_resp: ApiBarsResponse = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let bars = api_resp
            .bars
            .unwrap_or_default()
            .iter()
            .map(|b| Bar {
                symbol: symbol.to_string(),
                timestamp: std::time::SystemTime::now(),
                open: b.o,
                high: b.h,
                low: b.l,
                close: b.c,
                volume: b.v,
            })
            .collect();

        Ok(bars)
        // Ok(Vec::new())
    }

    pub async fn get_latest_bar(&self, symbol: &str) -> Result<Bar, DataError> {
        let bars = self.get_bars(symbol, "1Min", 1).await?;
        bars.into_iter()
            .next()
            .ok_or_else(|| DataError::NotFound(format!("No bars for {}", symbol)))
    }
}
