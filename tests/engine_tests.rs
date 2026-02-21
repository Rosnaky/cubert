use std::sync::Arc;
use tempfile::tempdir;

use cubert::broker::paper::PaperBroker;
use cubert::config::RiskConfig;
use cubert::logging::{Logger, Level};
use cubert::storage::Storage;
use cubert::strategy::{StrategyParams, StrategySettings};
use cubert::types::Bar;

// Mock data provider for testing
struct MockMarketData {
    prices: std::collections::HashMap<String, f64>,
}

impl MockMarketData {
    fn new() -> Self {
        let mut prices = std::collections::HashMap::new();
        prices.insert("AAPL".to_string(), 150.0);
        prices.insert("MSFT".to_string(), 300.0);
        Self { prices }
    }
}

fn default_risk_config() -> RiskConfig {
    RiskConfig {
        max_position_pct: 0.10,
        max_drawdown_pct: 0.05,
        max_daily_trades: 10,
        max_total_exposure: 0.80,
        max_loss_per_trade: 0.02,
    }
}

#[tokio::test]
async fn test_full_trade_cycle() {
    // Use in-memory database for tests
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account(100_000.0).await.unwrap();
    
    // Verify initial state
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 100_000.0);
    assert_eq!(account.equity, 100_000.0);
    
    // Simulate a buy
    let buy_qty = 100.0;
    let buy_price = 150.0;
    let order_value = buy_qty * buy_price;
    
    storage.deduct_cash(order_value).await.unwrap();
    storage.upsert_position("AAPL", buy_qty, buy_price, buy_price).await.unwrap();
    storage.insert_trade("AAPL", "buy", buy_qty, buy_price, Some("test-1"), Some("test")).await.unwrap();
    
    // Verify after buy
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 85_000.0);
    assert_eq!(account.equity, 100_000.0);  // Cash + position
    
    // Simulate price increase
    storage.update_position_price("AAPL", 160.0).await.unwrap();
    
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.equity, 101_000.0);  // $85k cash + $16k position
    
    // Simulate a sell
    let sell_qty = 100.0;
    let sell_price = 160.0;
    let sell_value = sell_qty * sell_price;
    
    storage.add_cash(sell_value).await.unwrap();
    storage.delete_position("AAPL").await.unwrap();
    storage.insert_trade("AAPL", "sell", sell_qty, sell_price, Some("test-2"), Some("test")).await.unwrap();
    
    // Verify after sell
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 101_000.0);
    assert_eq!(account.equity, 101_000.0);
    
    // Verify trades recorded
    let trades = storage.get_trades(10).await.unwrap();
    assert_eq!(trades.len(), 2);
}

#[tokio::test]
async fn test_multiple_positions() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account(100_000.0).await.unwrap();
    
    // Buy AAPL
    storage.deduct_cash(15_000.0).await.unwrap();
    storage.upsert_position("AAPL", 100.0, 150.0, 150.0).await.unwrap();
    
    // Buy MSFT
    storage.deduct_cash(30_000.0).await.unwrap();
    storage.upsert_position("MSFT", 100.0, 300.0, 300.0).await.unwrap();
    
    // Verify
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 55_000.0);
    assert_eq!(account.equity, 100_000.0);
    
    let positions = storage.get_positions().await.unwrap();
    assert_eq!(positions.len(), 2);
}

#[tokio::test]
async fn test_position_averaging() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account(100_000.0).await.unwrap();
    
    // First buy: 100 shares at $100
    storage.deduct_cash(10_000.0).await.unwrap();
    storage.upsert_position("AAPL", 100.0, 100.0, 100.0).await.unwrap();
    
    // Second buy: 100 shares at $120 (average in)
    storage.deduct_cash(12_000.0).await.unwrap();
    
    // Calculate new average
    let pos = storage.get_position("AAPL").await.unwrap().unwrap();
    let total_qty = pos.quantity + 100.0;
    let total_cost = (pos.quantity * pos.avg_entry_price) + 12_000.0;
    let new_avg = total_cost / total_qty;
    
    storage.upsert_position("AAPL", total_qty, new_avg, 120.0).await.unwrap();
    
    // Verify
    let pos = storage.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(pos.quantity, 200.0);
    assert_eq!(pos.avg_entry_price, 110.0);  // ($10k + $12k) / 200 shares
}
