use cubert::broker::Broker;
use cubert::broker::paper::PaperBroker;
use cubert::types::{Order, OrderType, Side};

#[tokio::test]
async fn test_paper_broker_buy_order() {
    let broker = PaperBroker::new(100_000.0);
    broker.set_price("AAPL", 150.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };

    let order_id = broker.submit_order(&order).await.unwrap();
    assert!(order_id.starts_with("PAPER-"));

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 85_000.0); // 100k - (100 * 150)
    assert_eq!(account.equity, 100_000.0);

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.quantity, 100.0);
    assert_eq!(position.avg_entry_price, 150.0);
}

#[tokio::test]
async fn test_paper_broker_sell_order() {
    let broker = PaperBroker::new(100_000.0);
    broker.set_price("AAPL", 150.0);

    // First buy
    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

    // Update price and sell
    broker.set_price("AAPL", 160.0);

    let sell_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Sell,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&sell_order).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 101_000.0); // 85k + (100 * 160)
    assert_eq!(account.equity, 101_000.0);

    let position = broker.get_position("AAPL").await.unwrap();
    assert!(position.is_none());
}

#[tokio::test]
async fn test_paper_broker_insufficient_funds() {
    let broker = PaperBroker::new(1_000.0);
    broker.set_price("AAPL", 150.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0, // Would cost $15k
        order_type: OrderType::Market,
    };

    let result = broker.submit_order(&order).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_paper_broker_insufficient_shares() {
    let broker = PaperBroker::new(100_000.0);
    broker.set_price("AAPL", 150.0);

    // Buy 50 shares
    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 50.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

    // Try to sell 100
    let sell_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Sell,
        quantity: 100.0,
        order_type: OrderType::Market,
    };

    let result = broker.submit_order(&sell_order).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_paper_broker_position_averaging() {
    let broker = PaperBroker::new(100_000.0);

    // Buy 100 @ $100
    broker.set_price("AAPL", 100.0);
    let order1 = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order1).await.unwrap();

    // Buy 100 @ $120
    broker.set_price("AAPL", 120.0);
    let order2 = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order2).await.unwrap();

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.quantity, 200.0);
    assert_eq!(position.avg_entry_price, 110.0); // ($10k + $12k) / 200
}

#[tokio::test]
async fn test_paper_broker_update_prices() {
    let broker = PaperBroker::new(100_000.0);
    broker.set_price("AAPL", 150.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order).await.unwrap();

    // Update prices
    broker.set_price("AAPL", 200.0);
    broker.update_prices(&["AAPL".to_string()]).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.equity, 105_000.0); // 85k cash + 100 * 200

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.current_price, 200.0);
}

#[tokio::test]
async fn test_paper_broker_multiple_positions() {
    let broker = PaperBroker::new(100_000.0);

    broker.set_price("AAPL", 150.0);
    broker.set_price("MSFT", 300.0);

    let order1 = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order1).await.unwrap();

    let order2 = Order {
        symbol: "MSFT".to_string(),
        side: Side::Buy,
        quantity: 50.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order2).await.unwrap();

    let positions = broker.get_positions().await.unwrap();
    assert_eq!(positions.len(), 2);

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 70_000.0); // 100k - 15k - 15k
    assert_eq!(account.equity, 100_000.0);
}

#[tokio::test]
async fn test_paper_broker_partial_sell() {
    let broker = PaperBroker::new(100_000.0);
    broker.set_price("AAPL", 100.0);

    // Buy 100 shares
    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

    // Sell 40 shares
    let sell_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Sell,
        quantity: 40.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&sell_order).await.unwrap();

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.quantity, 60.0);

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 94_000.0); // 90k + 4k
}

#[tokio::test]
async fn test_paper_broker_limit_order() {
    let broker = PaperBroker::new(100_000.0);
    // Don't set market price - limit order uses its own price

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Limit { price: 145.0 },
    };

    broker.submit_order(&order).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 85_500.0); // 100k - (100 * 145)

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.avg_entry_price, 145.0);
}
