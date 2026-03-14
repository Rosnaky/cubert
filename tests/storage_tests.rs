use cubert::storage::Storage;

async fn setup_test_storage() -> Storage {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage
}

#[tokio::test]
async fn test_account_init() {
    let storage = setup_test_storage().await;
    storage.init_account("a", 100_000.0).await.unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 100_000.0);
    assert_eq!(account.equity, 100_000.0);
}

#[tokio::test]
async fn test_account_init_idempotent() {
    let storage = setup_test_storage().await;

    storage.init_account("a", 100_000.0).await.unwrap();
    storage.init_account("b", 50_000.0).await.unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 100_000.0);
}

#[tokio::test]
async fn test_cash_operations() {
    let storage = setup_test_storage().await;
    storage.init_account("a", 10_000.0).await.unwrap();

    let remaining = storage.deduct_cash("a", 3_000.0).await.unwrap();
    assert_eq!(remaining, 7_000.0);

    let new_total = storage.add_cash("a", 1_000.0).await.unwrap();
    assert_eq!(new_total, 8_000.0);
}

#[tokio::test]
async fn test_insufficient_funds() {
    let storage = setup_test_storage().await;
    storage.init_account("a", 1_000.0).await.unwrap();

    let result = storage.deduct_cash("a", 5_000.0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_position_crud() {
    let storage = setup_test_storage().await;
    storage.init_account("a", 100_000.0).await.unwrap();

    storage
        .upsert_position("a", "AAPL", 100.0, 150.0, 155.0)
        .await
        .unwrap();

    let pos = storage.get_position("a", "AAPL").await.unwrap().unwrap();
    assert_eq!(pos.symbol, "AAPL");
    assert_eq!(pos.quantity, 100.0);
    assert_eq!(pos.avg_entry_price, 150.0);

    storage
        .upsert_position("a", "AAPL", 150.0, 152.0, 160.0)
        .await
        .unwrap();
    let pos = storage.get_position("a", "AAPL").await.unwrap().unwrap();
    assert_eq!(pos.quantity, 150.0);

    storage.delete_position("a", "AAPL").await.unwrap();
    let pos = storage.get_position("a", "AAPL").await.unwrap();
    assert!(pos.is_none());
}

#[tokio::test]
async fn test_equity_calculation() {
    let storage = setup_test_storage().await;
    storage.init_account("a", 50_000.0).await.unwrap();

    storage
        .upsert_position("a", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();

    let account = storage.get_account("a").await.unwrap();
    assert_eq!(account.cash, 50_000.0);
    assert_eq!(account.equity, 65_000.0);
}

#[tokio::test]
async fn test_trade_insert_and_query() {
    let storage = setup_test_storage().await;

    storage
        .insert_trade(
            "acct_trades",
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
            "acct_trades",
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
            "acct_trades",
            "AAPL",
            "sell",
            100.0,
            160.0,
            Some("order-3"),
            Some("momentum"),
        )
        .await
        .unwrap();

    let trades = storage.get_trades(10).await.unwrap();
    assert_eq!(trades.len(), 3);

    let aapl_trades = storage.get_trades_by_symbol("AAPL", 10).await.unwrap();
    assert_eq!(aapl_trades.len(), 2);

    let acct_trades = storage
        .get_trades_by_account("acct_trades", 10)
        .await
        .unwrap();
    assert_eq!(acct_trades.len(), 3);
}

#[tokio::test]
async fn test_signal_insert_and_query() {
    let storage = setup_test_storage().await;

    storage
        .insert_signal("acct_signals", "AAPL", "buy", 0.8, "momentum")
        .await
        .unwrap();
    storage
        .insert_signal("acct_signals", "MSFT", "sell", 0.6, "mean_reversion")
        .await
        .unwrap();

    let signals = storage.get_signals(10).await.unwrap();
    assert_eq!(signals.len(), 2);

    let acct_signals = storage
        .get_signals_by_account("acct_signals", 10)
        .await
        .unwrap();
    assert_eq!(acct_signals.len(), 2);
}

#[tokio::test]
async fn test_account_snapshot() {
    let storage = setup_test_storage().await;

    storage
        .insert_account_snapshot("acct_snap", 100_000.0, 80_000.0, 80_000.0)
        .await
        .unwrap();
    storage
        .insert_account_snapshot("acct_snap", 101_000.0, 81_000.0, 81_000.0)
        .await
        .unwrap();

    let history = storage.get_account_history("acct_snap", 10).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].equity, 101_000.0);
}

#[tokio::test]
async fn test_strategy_crud() {
    let storage = setup_test_storage().await;

    let params = cubert::strategy::StrategyParams::Momentum {
        lookback_period: 20,
        threshold: 0.03,
        startup_lookback: "1Hour".to_string(),
        startup_bar_limit: 50,
    };

    let id = storage
        .insert_strategy("fast_momentum", &params)
        .await
        .unwrap();

    let strat = storage.get_strategy_by_id(&id).await.unwrap();
    assert_eq!(strat.name, "fast_momentum");
    assert_eq!(strat.strategy_type, "momentum");

    let all = storage.get_strategies().await.unwrap();
    assert_eq!(all.len(), 1);

    storage
        .update_strategy(&id, "slow_momentum", "momentum", &strat.params_json)
        .await
        .unwrap();

    let updated = storage.get_strategy_by_id(&id).await.unwrap();
    assert_eq!(updated.name, "slow_momentum");

    storage.delete_strategy(&id).await.unwrap();
    let all = storage.get_strategies().await.unwrap();
    assert_eq!(all.len(), 0);
}

#[tokio::test]
async fn test_account_strategy_assignment() {
    let storage = setup_test_storage().await;

    let params = cubert::strategy::StrategyParams::Momentum {
        lookback_period: 10,
        threshold: 0.02,
        startup_lookback: "5Min".to_string(),
        startup_bar_limit: 100,
    };

    let strat_id = storage
        .insert_strategy("test_strat", &params)
        .await
        .unwrap();

    storage
        .assign_strategy_to_account(
            "paper_aggressive",
            &strat_id,
            &["AAPL".to_string(), "TSLA".to_string()],
        )
        .await
        .unwrap();

    let assignments = storage
        .get_strategies_for_account("paper_aggressive")
        .await
        .unwrap();
    assert_eq!(assignments.len(), 1);

    let (strat, assignment) = &assignments[0];
    assert_eq!(strat.name, "test_strat");
    assert_eq!(assignment.account_id, "paper_aggressive");

    let account_ids = storage.get_active_account_ids().await.unwrap();
    assert_eq!(account_ids.len(), 1);
    assert_eq!(account_ids[0], "paper_aggressive");

    storage
        .toggle_strategy_enabled("paper_aggressive", &strat_id, false)
        .await
        .unwrap();

    let account_ids = storage.get_active_account_ids().await.unwrap();
    assert_eq!(account_ids.len(), 0);

    storage
        .unassign_strategy_from_account("paper_aggressive", &strat_id)
        .await
        .unwrap();

    let assignments = storage
        .get_strategies_for_account("paper_aggressive")
        .await
        .unwrap();
    assert_eq!(assignments.len(), 0);
}
