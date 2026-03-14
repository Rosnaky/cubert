use cubert::broker::Broker;
use cubert::broker::paper::PaperBroker;
use cubert::types::{Order, OrderType, Side};

#[tokio::test]
async fn test_paper_broker_buy_order() {
    let broker = PaperBroker::new("test_buy", 100_000.0);
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
    assert_eq!(account.cash, 85_000.0);
    assert_eq!(account.equity, 100_000.0);

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.quantity, 100.0);
    assert_eq!(position.avg_entry_price, 150.0);
}

#[tokio::test]
async fn test_paper_broker_sell_order() {
    let broker = PaperBroker::new("test_sell", 100_000.0);
    broker.set_price("AAPL", 150.0);

    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

    broker.set_price("AAPL", 160.0);

    let sell_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Sell,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&sell_order).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 101_000.0);
    assert_eq!(account.equity, 101_000.0);

    let position = broker.get_position("AAPL").await.unwrap();
    assert!(position.is_none());
}

#[tokio::test]
async fn test_paper_broker_insufficient_funds() {
    let broker = PaperBroker::new("test_funds", 1_000.0);
    broker.set_price("AAPL", 150.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };

    let result = broker.submit_order(&order).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_paper_broker_insufficient_shares() {
    let broker = PaperBroker::new("test_shares", 100_000.0);
    broker.set_price("AAPL", 150.0);

    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 50.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

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
    let broker = PaperBroker::new("test_avg", 100_000.0);

    broker.set_price("AAPL", 100.0);
    let order1 = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order1).await.unwrap();

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
    assert_eq!(position.avg_entry_price, 110.0);
}

#[tokio::test]
async fn test_paper_broker_update_prices() {
    let broker = PaperBroker::new("test_prices", 100_000.0);
    broker.set_price("AAPL", 150.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&order).await.unwrap();

    broker.set_price("AAPL", 200.0);
    broker.update_prices(&["AAPL".to_string()]).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.equity, 105_000.0);

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.current_price, 200.0);
}

#[tokio::test]
async fn test_paper_broker_multiple_positions() {
    let broker = PaperBroker::new("test_multi", 100_000.0);

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
    assert_eq!(account.cash, 70_000.0);
    assert_eq!(account.equity, 100_000.0);
}

#[tokio::test]
async fn test_paper_broker_partial_sell() {
    let broker = PaperBroker::new("test_partial", 100_000.0);
    broker.set_price("AAPL", 100.0);

    let buy_order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Market,
    };
    broker.submit_order(&buy_order).await.unwrap();

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
    assert_eq!(account.cash, 94_000.0);
}

#[tokio::test]
async fn test_paper_broker_limit_order() {
    let broker = PaperBroker::new("test_limit", 100_000.0);

    let order = Order {
        symbol: "AAPL".to_string(),
        side: Side::Buy,
        quantity: 100.0,
        order_type: OrderType::Limit { price: 145.0 },
    };

    broker.submit_order(&order).await.unwrap();

    let account = broker.get_account().await.unwrap();
    assert_eq!(account.cash, 85_500.0);

    let position = broker.get_position("AAPL").await.unwrap().unwrap();
    assert_eq!(position.avg_entry_price, 145.0);
}
