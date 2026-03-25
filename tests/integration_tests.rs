use cubert::broker::Broker;
use cubert::broker::paper::PaperBroker;
use cubert::config::RiskConfig;
use cubert::risk::RiskManager;
use cubert::storage::Storage;
use cubert::types::{Side, Signal};

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
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account("a", 100_000.0).await.unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 100_000.0);
    assert_eq!(account.equity, 100_000.0);

    let buy_qty = 100.0;
    let buy_price = 150.0;
    let order_value = buy_qty * buy_price;

    storage.deduct_cash("a", order_value).await.unwrap();
    storage
        .upsert_position("a", "AAPL", buy_qty, buy_price, buy_price)
        .await
        .unwrap();
    storage
        .insert_trade(
            "acct_cycle",
            "AAPL",
            "buy",
            buy_qty,
            buy_price,
            Some("test-1"),
            Some("test"),
        )
        .await
        .unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 85_000.0);
    assert_eq!(account.equity, 100_000.0);

    storage
        .update_position_price("a", "AAPL", 160.0)
        .await
        .unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.equity, 101_000.0);

    let sell_qty = 100.0;
    let sell_price = 160.0;
    let sell_value = sell_qty * sell_price;

    storage.add_cash("a", sell_value).await.unwrap();
    storage.delete_position("a", "AAPL").await.unwrap();
    storage
        .insert_trade(
            "acct_cycle",
            "AAPL",
            "sell",
            sell_qty,
            sell_price,
            Some("test-2"),
            Some("test"),
        )
        .await
        .unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 101_000.0);
    assert_eq!(account.equity, 101_000.0);

    let trades = storage.get_trades(10).await.unwrap();
    assert_eq!(trades.len(), 2);
}

#[tokio::test]
async fn test_multiple_positions() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account("a", 100_000.0).await.unwrap();

    storage.deduct_cash("a", 15_000.0).await.unwrap();
    storage
        .upsert_position("a", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();

    storage.deduct_cash("a", 30_000.0).await.unwrap();
    storage
        .upsert_position("a", "MSFT", 100.0, 300.0, 300.0)
        .await
        .unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 55_000.0);
    assert_eq!(account.equity, 100_000.0);

    let positions = storage.get_positions("a").await.unwrap();
    assert_eq!(positions.len(), 2);
}

#[tokio::test]
async fn test_position_averaging() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account("a", 100_000.0).await.unwrap();

    storage.deduct_cash("a", 10_000.0).await.unwrap();
    storage
        .upsert_position("a", "AAPL", 100.0, 100.0, 100.0)
        .await
        .unwrap();

    storage.deduct_cash("a", 12_000.0).await.unwrap();

    let pos = storage.get_position("a", "AAPL").await.unwrap().unwrap();
    let total_qty = pos.quantity + 100.0;
    let total_cost = (pos.quantity * pos.avg_entry_price) + 12_000.0;
    let new_avg = total_cost / total_qty;

    storage
        .upsert_position("a", "AAPL", total_qty, new_avg, 120.0)
        .await
        .unwrap();

    let pos = storage.get_position("a", "AAPL").await.unwrap().unwrap();
    assert_eq!(pos.quantity, 200.0);
    assert_eq!(pos.avg_entry_price, 110.0);
}

#[tokio::test]
async fn test_risk_manager_with_broker() {
    let broker = PaperBroker::new("acct_risk", 100_000.0);
    let mut rm = RiskManager::new(default_risk_config());

    broker.set_price("AAPL", 150.0);

    let account = broker.get_account().await.unwrap();
    let positions = broker.get_positions().await.unwrap();

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    let order = rm.evaluate_signal(&signal, &account, &positions, 150.0);
    assert!(order.is_some());

    let order = order.unwrap();
    let order_id = broker.submit_order(&order).await.unwrap();
    assert!(order_id.starts_with("PAPER-"));

    let new_account = broker.get_account().await.unwrap();
    assert!(new_account.cash < account.cash);

    let position = broker.get_position("AAPL").await.unwrap();
    assert!(position.is_some());
}

#[tokio::test]
async fn test_signal_to_trade_flow() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account("a", 100_000.0).await.unwrap();

    let broker = PaperBroker::new("acct_flow", 100_000.0);
    let mut rm = RiskManager::new(default_risk_config());

    broker.set_price("AAPL", 150.0);

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    storage
        .insert_signal("acct_flow", "AAPL", "buy", 0.8, "momentum")
        .await
        .unwrap();

    let account = broker.get_account().await.unwrap();
    let positions = broker.get_positions().await.unwrap();

    let order = rm
        .evaluate_signal(&signal, &account, &positions, 150.0)
        .unwrap();

    let order_id = broker.submit_order(&order).await.unwrap();

    let side = match order.side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    };
    storage
        .insert_trade(
            "acct_flow",
            "AAPL",
            side,
            order.quantity,
            150.0,
            Some(&order_id),
            Some("momentum"),
        )
        .await
        .unwrap();

    let account = broker.get_account().await.unwrap();
    storage
        .insert_account_snapshot(
            "acct_flow",
            account.equity,
            account.cash,
            account.buying_power,
        )
        .await
        .unwrap();

    let signals = storage.get_signals(10).await.unwrap();
    assert_eq!(signals.len(), 1);

    let trades = storage.get_trades(10).await.unwrap();
    assert_eq!(trades.len(), 1);

    let history = storage.get_account_history("acct_flow", 10).await.unwrap();
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn test_dual_database_separation() {
    let broker_storage = Storage::connect("sqlite::memory:").await.unwrap();
    broker_storage.migrate().await.unwrap();
    broker_storage.init_account("a", 100_000.0).await.unwrap();

    let engine_storage = Storage::connect("sqlite::memory:").await.unwrap();
    engine_storage.migrate().await.unwrap();

    broker_storage.deduct_cash("a", 15_000.0).await.unwrap();
    broker_storage
        .upsert_position("a", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();
    broker_storage
        .insert_trade(
            "acct_dual",
            "AAPL",
            "buy",
            100.0,
            150.0,
            Some("order-1"),
            None,
        )
        .await
        .unwrap();

    engine_storage
        .insert_signal("acct_dual", "AAPL", "buy", 0.8, "momentum")
        .await
        .unwrap();
    engine_storage
        .insert_trade(
            "acct_dual",
            "AAPL",
            "buy",
            100.0,
            150.0,
            Some("order-1"),
            Some("momentum"),
        )
        .await
        .unwrap();

    let account = broker_storage.get_account("a").await.unwrap();
    engine_storage
        .insert_account_snapshot(
            "acct_dual",
            account.equity,
            account.cash,
            account.buying_power,
        )
        .await
        .unwrap();

    let broker_trades = broker_storage.get_trades(10).await.unwrap();
    assert_eq!(broker_trades.len(), 1);
    assert!(broker_trades[0].strategy.is_none());

    let engine_trades = engine_storage.get_trades(10).await.unwrap();
    assert_eq!(engine_trades.len(), 1);
    assert_eq!(engine_trades[0].strategy.as_ref().unwrap(), "momentum");

    let engine_signals = engine_storage.get_signals(10).await.unwrap();
    assert_eq!(engine_signals.len(), 1);

    let broker_signals = broker_storage.get_signals(10).await.unwrap();
    assert_eq!(broker_signals.len(), 0);
}
