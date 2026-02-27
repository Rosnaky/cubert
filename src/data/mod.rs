use std::collections::HashMap;

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
    bars: Option<HashMap<String, ApiBar>>,
    symbol: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiLatestBarResponse {
    bar: Option<ApiBar>,
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct ApiMultiBarsResponse {
    bars: Option<HashMap<String, ApiBar>>,
}

#[derive(Debug, Deserialize)]
struct ApiMultiBarsHistoryResponse {
    bars: Option<HashMap<String, Vec<ApiBar>>>,
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

    pub async fn get_bars_batch(
        &self,
        symbols: &[String],
        timeframe: &str,
        limit: u32,
    ) -> Result<HashMap<String, Vec<Bar>>, DataError> {
        use chrono::{Duration, Utc};

        let end = Utc::now();
        let start = end - Duration::days(7);
        let symbols_param = symbols.join(",");

        let url = format!(
            "{}/v2/stocks/bars?symbols={}&timeframe={}&limit={}&start={}&feed=iex",
            self.data_endpoint,
            symbols_param,
            timeframe,
            limit,
            start.format("%Y-%m-%dT%H:%M:%SZ")
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

        let api_resp: ApiMultiBarsHistoryResponse = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let mut result = HashMap::new();
        if let Some(bars_map) = api_resp.bars {
            for (symbol, api_bars) in bars_map {
                let bars: Vec<Bar> = api_bars
                    .into_iter()
                    .map(|b| Bar {
                        symbol: symbol.clone(),
                        timestamp: std::time::SystemTime::now(),
                        open: b.o,
                        high: b.h,
                        low: b.l,
                        close: b.c,
                        volume: b.v,
                    })
                    .collect();
                result.insert(symbol, bars);
            }
        }

        Ok(result)
    }

    pub async fn get_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<Vec<Bar>, DataError> {
        let symbols = vec![symbol.to_string()];
        let mut result = self.get_bars_batch(&symbols, timeframe, limit).await?;
        Ok(result.remove(symbol).unwrap_or_default())
    }

    pub async fn get_latest_bars(
        &self,
        symbols: &[String],
    ) -> Result<HashMap<String, Bar>, DataError> {
        let symbols_param = symbols.join(",");
        let url = format!(
            "{}/v2/stocks/bars/latest?symbols={}&feed=iex",
            self.data_endpoint, symbols_param
        );

        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| DataError::ConnectionFailed(e.to_string()))?;

        if resp.status().is_success() {
            let api_resp: ApiMultiBarsResponse = resp
                .json()
                .await
                .map_err(|e| DataError::ParseError(e.to_string()))?;

            if let Some(bars_map) = api_resp.bars {
                let mut result = HashMap::new();
                for (symbol, b) in bars_map {
                    result.insert(
                        symbol.clone(),
                        Bar {
                            symbol,
                            timestamp: std::time::SystemTime::now(),
                            open: b.o,
                            high: b.h,
                            low: b.l,
                            close: b.c,
                            volume: b.v,
                        },
                    );
                }
                return Ok(result);
            }
        }

        // Fallback: get historical bars for all symbols
        let mut result = HashMap::new();
        for symbol in symbols {
            if let Ok(bars) = self.get_bars(symbol, "1Day", 1).await
                && let Some(bar) = bars.into_iter().next()
            {
                result.insert(symbol.clone(), bar);
            }
        }
        Ok(result)
    }

    pub async fn get_latest_bar(&self, symbol: &str) -> Result<Bar, DataError> {
        let symbols = vec![symbol.to_string()];
        let mut result = self.get_latest_bars(&symbols).await?;
        result
            .remove(symbol)
            .ok_or_else(|| DataError::NotFound(format!("No bar for {}", symbol)))
    }
}
