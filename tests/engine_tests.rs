use cubert::storage::Storage;

const ACCT: &str = "test_paper";

async fn setup() -> Storage {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();
    storage.init_account(ACCT, 100_000.0).await.unwrap();
    storage
}

#[tokio::test]
async fn test_full_trade_cycle() {
    let storage = setup().await;

    let account = storage.get_account(ACCT).await.unwrap();
    assert_eq!(account.cash, 100_000.0);
    assert_eq!(account.equity, 100_000.0);

    let buy_qty = 100.0;
    let buy_price = 150.0;
    let order_value = buy_qty * buy_price;

    storage.deduct_cash(ACCT, order_value).await.unwrap();
    storage
        .upsert_position(ACCT, "AAPL", buy_qty, buy_price, buy_price)
        .await
        .unwrap();
    storage
        .insert_trade(
            ACCT,
            "AAPL",
            "buy",
            buy_qty,
            buy_price,
            Some("test-1"),
            Some("momentum"),
        )
        .await
        .unwrap();

    let account = storage.get_account(ACCT).await.unwrap();
    assert_eq!(account.cash, 85_000.0);
    assert_eq!(account.equity, 100_000.0);

    storage
        .update_position_price(ACCT, "AAPL", 160.0)
        .await
        .unwrap();

    let account = storage.get_account(ACCT).await.unwrap();
    assert_eq!(account.equity, 101_000.0);

    let sell_qty = 100.0;
    let sell_price = 160.0;
    let sell_value = sell_qty * sell_price;

    storage.add_cash(ACCT, sell_value).await.unwrap();
    storage.delete_position(ACCT, "AAPL").await.unwrap();
    storage
        .insert_trade(
            ACCT,
            "AAPL",
            "sell",
            sell_qty,
            sell_price,
            Some("test-2"),
            Some("momentum"),
        )
        .await
        .unwrap();

    let account = storage.get_account(ACCT).await.unwrap();
    assert_eq!(account.cash, 101_000.0);
    assert_eq!(account.equity, 101_000.0);

    let trades = storage.get_trades_by_account(ACCT, 10).await.unwrap();
    assert_eq!(trades.len(), 2);
}

#[tokio::test]
async fn test_multiple_positions() {
    let storage = setup().await;

    storage.deduct_cash(ACCT, 15_000.0).await.unwrap();
    storage
        .upsert_position(ACCT, "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();

    storage.deduct_cash(ACCT, 30_000.0).await.unwrap();
    storage
        .upsert_position(ACCT, "MSFT", 100.0, 300.0, 300.0)
        .await
        .unwrap();

    let account = storage.get_account(ACCT).await.unwrap();
    assert_eq!(account.cash, 55_000.0);
    assert_eq!(account.equity, 100_000.0);

    let positions = storage.get_positions(ACCT).await.unwrap();
    assert_eq!(positions.len(), 2);
}

#[tokio::test]
async fn test_position_averaging() {
    let storage = setup().await;

    storage.deduct_cash(ACCT, 10_000.0).await.unwrap();
    storage
        .upsert_position(ACCT, "AAPL", 100.0, 100.0, 100.0)
        .await
        .unwrap();

    storage.deduct_cash(ACCT, 12_000.0).await.unwrap();

    let pos = storage.get_position(ACCT, "AAPL").await.unwrap().unwrap();
    let total_qty = pos.quantity + 100.0;
    let total_cost = (pos.quantity * pos.avg_entry_price) + 12_000.0;
    let new_avg = total_cost / total_qty;

    storage
        .upsert_position(ACCT, "AAPL", total_qty, new_avg, 120.0)
        .await
        .unwrap();

    let pos = storage.get_position(ACCT, "AAPL").await.unwrap().unwrap();
    assert_eq!(pos.quantity, 200.0);
    assert_eq!(pos.avg_entry_price, 110.0);
}

#[tokio::test]
async fn test_account_isolation() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage.init_account("aggressive", 100_000.0).await.unwrap();
    storage
        .init_account("conservative", 50_000.0)
        .await
        .unwrap();

    storage.deduct_cash("aggressive", 15_000.0).await.unwrap();
    storage
        .upsert_position("aggressive", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();

    storage.deduct_cash("conservative", 6_000.0).await.unwrap();
    storage
        .upsert_position("conservative", "MSFT", 20.0, 300.0, 300.0)
        .await
        .unwrap();

    let agg = storage.get_account("aggressive").await.unwrap();
    assert_eq!(agg.cash, 85_000.0);
    assert_eq!(agg.equity, 100_000.0);
    assert_eq!(agg.id, "aggressive");

    let con = storage.get_account("conservative").await.unwrap();
    assert_eq!(con.cash, 44_000.0);
    assert_eq!(con.equity, 50_000.0);
    assert_eq!(con.id, "conservative");

    let agg_positions = storage.get_positions("aggressive").await.unwrap();
    assert_eq!(agg_positions.len(), 1);
    assert_eq!(agg_positions[0].symbol, "AAPL");

    let con_positions = storage.get_positions("conservative").await.unwrap();
    assert_eq!(con_positions.len(), 1);
    assert_eq!(con_positions[0].symbol, "MSFT");
}

#[tokio::test]
async fn test_same_symbol_different_accounts() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage.init_account("acct_a", 100_000.0).await.unwrap();
    storage.init_account("acct_b", 100_000.0).await.unwrap();

    storage
        .upsert_position("acct_a", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();
    storage
        .upsert_position("acct_b", "AAPL", 50.0, 155.0, 155.0)
        .await
        .unwrap();

    let pos_a = storage
        .get_position("acct_a", "AAPL")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pos_a.quantity, 100.0);
    assert_eq!(pos_a.avg_entry_price, 150.0);

    let pos_b = storage
        .get_position("acct_b", "AAPL")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pos_b.quantity, 50.0);
    assert_eq!(pos_b.avg_entry_price, 155.0);

    storage.delete_position("acct_a", "AAPL").await.unwrap();

    let pos_a = storage.get_position("acct_a", "AAPL").await.unwrap();
    assert!(pos_a.is_none());

    let pos_b = storage
        .get_position("acct_b", "AAPL")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pos_b.quantity, 50.0);
}

#[tokio::test]
async fn test_trades_scoped_to_account() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage
        .insert_trade(
            "acct_x",
            "AAPL",
            "buy",
            100.0,
            150.0,
            Some("o1"),
            Some("momentum"),
        )
        .await
        .unwrap();
    storage
        .insert_trade(
            "acct_x",
            "MSFT",
            "buy",
            50.0,
            300.0,
            Some("o2"),
            Some("momentum"),
        )
        .await
        .unwrap();
    storage
        .insert_trade(
            "acct_y",
            "AAPL",
            "buy",
            25.0,
            148.0,
            Some("o3"),
            Some("mean_rev"),
        )
        .await
        .unwrap();

    let all = storage.get_trades(10).await.unwrap();
    assert_eq!(all.len(), 3);

    let x_trades = storage.get_trades_by_account("acct_x", 10).await.unwrap();
    assert_eq!(x_trades.len(), 2);

    let y_trades = storage.get_trades_by_account("acct_y", 10).await.unwrap();
    assert_eq!(y_trades.len(), 1);
    assert_eq!(y_trades[0].symbol, "AAPL");
    assert_eq!(y_trades[0].strategy.as_ref().unwrap(), "mean_rev");
}

#[tokio::test]
async fn test_signals_scoped_to_account() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage
        .insert_signal("acct_alpha", "AAPL", "buy", 0.9, "momentum")
        .await
        .unwrap();
    storage
        .insert_signal("acct_alpha", "TSLA", "sell", 0.7, "momentum")
        .await
        .unwrap();
    storage
        .insert_signal("acct_beta", "GOOGL", "buy", 0.5, "mean_rev")
        .await
        .unwrap();

    let alpha = storage
        .get_signals_by_account("acct_alpha", 10)
        .await
        .unwrap();
    assert_eq!(alpha.len(), 2);

    let beta = storage
        .get_signals_by_account("acct_beta", 10)
        .await
        .unwrap();
    assert_eq!(beta.len(), 1);
    assert_eq!(beta[0].symbol, "GOOGL");
}

#[tokio::test]
async fn test_snapshots_scoped_to_account() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage
        .insert_account_snapshot("acct_one", 100_000.0, 80_000.0, 80_000.0)
        .await
        .unwrap();
    storage
        .insert_account_snapshot("acct_one", 102_000.0, 82_000.0, 82_000.0)
        .await
        .unwrap();
    storage
        .insert_account_snapshot("acct_two", 50_000.0, 50_000.0, 50_000.0)
        .await
        .unwrap();

    let one = storage.get_account_history("acct_one", 10).await.unwrap();
    assert_eq!(one.len(), 2);
    assert_eq!(one[0].equity, 102_000.0);

    let two = storage.get_account_history("acct_two", 10).await.unwrap();
    assert_eq!(two.len(), 1);
    assert_eq!(two[0].equity, 50_000.0);
}

#[tokio::test]
async fn test_init_account_idempotent() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage.init_account("paper_1", 100_000.0).await.unwrap();
    storage.init_account("paper_1", 50_000.0).await.unwrap();

    let account = storage.get_account("paper_1").await.unwrap();
    assert_eq!(account.cash, 100_000.0);
}

#[tokio::test]
async fn test_insufficient_funds_per_account() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage.init_account("rich", 100_000.0).await.unwrap();
    storage.init_account("poor", 1_000.0).await.unwrap();

    let result = storage.deduct_cash("poor", 5_000.0).await;
    assert!(result.is_err());

    let rich = storage.get_account("rich").await.unwrap();
    assert_eq!(rich.cash, 100_000.0);
}

#[tokio::test]
async fn test_price_update_scoped_to_account() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.migrate().await.unwrap();

    storage.init_account("acct_a", 100_000.0).await.unwrap();
    storage.init_account("acct_b", 100_000.0).await.unwrap();

    storage
        .upsert_position("acct_a", "AAPL", 100.0, 150.0, 150.0)
        .await
        .unwrap();
    storage
        .upsert_position("acct_b", "AAPL", 200.0, 145.0, 145.0)
        .await
        .unwrap();

    storage
        .update_position_price("acct_a", "AAPL", 160.0)
        .await
        .unwrap();

    let pos_a = storage
        .get_position("acct_a", "AAPL")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pos_a.current_price, 160.0);

    let pos_b = storage
        .get_position("acct_b", "AAPL")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pos_b.current_price, 145.0);
}
