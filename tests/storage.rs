use cubert::storage::Storage;

async fn setup_test_storage() -> Storage {
    // Use in-memory database
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage
}

#[tokio::test]
async fn test_account_init() {
    let storage = setup_test_storage().await;

    // Init account
    storage.init_account(100_000.0).await.unwrap();

    // Get account
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 100_000.0);
    assert_eq!(account.equity, 100_000.0);
}

#[tokio::test]
async fn test_account_init_idempotent() {
    let storage = setup_test_storage().await;

    // Init twice with different amounts
    storage.init_account(100_000.0).await.unwrap();
    storage.init_account(50_000.0).await.unwrap(); // Should be ignored

    // Should still be first value
    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 100_000.0);
}

#[tokio::test]
async fn test_cash_operations() {
    let storage = setup_test_storage().await;
    storage.init_account(10_000.0).await.unwrap();

    // Deduct
    let remaining = storage.deduct_cash(3_000.0).await.unwrap();
    assert_eq!(remaining, 7_000.0);

    // Add
    let new_total = storage.add_cash(1_000.0).await.unwrap();
    assert_eq!(new_total, 8_000.0);
}

#[tokio::test]
async fn test_insufficient_funds() {
    let storage = setup_test_storage().await;
    storage.init_account(1_000.0).await.unwrap();

    // Try to deduct more than available
    let result = storage.deduct_cash(5_000.0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_position_crud() {
    let storage = setup_test_storage().await;
    storage.init_account(100_000.0).await.unwrap();

    // Create position
    storage
        .upsert_position("AAPL", 100.0, 150.0, 155.0)
        .await
        .unwrap();

    // Read position
    let pos = storage.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(pos.symbol, "AAPL");
    assert_eq!(pos.quantity, 100.0);
    assert_eq!(pos.avg_entry_price, 150.0);

    // Update position
    storage
        .upsert_position("AAPL", 150.0, 152.0, 160.0)
        .await
        .unwrap();
    let pos = storage.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(pos.quantity, 150.0);

    // Delete position
    storage.delete_position("AAPL").await.unwrap();
    let pos = storage.get_position("AAPL").await.unwrap();
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_equity_calculation() {
    let storage = setup_test_storage().await;
    storage.init_account(50_000.0).await.unwrap();

    // Add a position worth $15,000
    storage
        .upsert_position("AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();

    let account = storage.get_account().await.unwrap();
    assert_eq!(account.cash, 50_000.0);
    assert_eq!(account.equity, 65_000.0); // cash + position value
}

#[tokio::test]
async fn test_trade_insert_and_query() {
    let storage = setup_test_storage().await;

    // Insert trades
    storage
        .insert_trade(
            "AAPL",
            "buy",
            100.0,
            150.0,
            Some("order-1"),
            Some("momentum"),
        )
        .await
        .unwrap();
    storage
        .insert_trade(
            "MSFT",
            "buy",
            50.0,
            300.0,
            Some("order-2"),
            Some("momentum"),
        )
        .await
        .unwrap();
    storage
        .insert_trade(
            "AAPL",
            "sell",
            100.0,
            160.0,
            Some("order-3"),
            Some("momentum"),
        )
        .await
        .unwrap();

    // Query all trades
    let trades = storage.get_trades(10).await.unwrap();
    assert_eq!(trades.len(), 3);

    // Query by symbol
    let aapl_trades = storage.get_trades_by_symbol("AAPL", 10).await.unwrap();
    assert_eq!(aapl_trades.len(), 2);
}

#[tokio::test]
async fn test_signal_insert_and_query() {
    let storage = setup_test_storage().await;

    storage
        .insert_signal("AAPL", "buy", 0.8, "momentum")
        .await
        .unwrap();
    storage
        .insert_signal("MSFT", "sell", 0.6, "mean_reversion")
        .await
        .unwrap();

    let signals = storage.get_signals(10).await.unwrap();
    assert_eq!(signals.len(), 2);
}

#[tokio::test]
async fn test_account_snapshot() {
    let storage = setup_test_storage().await;

    storage
        .insert_account_snapshot(100_000.0, 80_000.0, 80_000.0)
        .await
        .unwrap();
    storage
        .insert_account_snapshot(101_000.0, 81_000.0, 81_000.0)
        .await
        .unwrap();

    let history = storage.get_account_history(10).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].equity, 101_000.0); // Most recent first
}
