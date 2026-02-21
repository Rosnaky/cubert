use cubert::config::RiskConfig;
use cubert::risk::RiskManager;
use cubert::types::{Account, OrderType, Position, Side, Signal};

fn default_risk_config() -> RiskConfig {
    RiskConfig {
        max_position_pct: 0.10, // 10% max per position
        max_drawdown_pct: 0.05,
        max_daily_trades: 10,
        max_total_exposure: 0.80, // 80% max total
        max_loss_per_trade: 0.02,
    }
}

fn default_account() -> Account {
    Account {
        equity: 100_000.0,
        cash: 100_000.0,
        buying_power: 100_000.0,
    }
}

#[test]
fn test_risk_manager_approves_buy() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    let order = rm.evaluate_signal(&signal, &account, &positions, 150.0);

    assert!(order.is_some(), "Should approve buy signal");
    let order = order.unwrap();
    assert_eq!(order.symbol, "AAPL");
    assert!(matches!(order.side, Side::Buy));
    assert!(order.quantity > 0.0);
}

#[test]
fn test_risk_manager_rejects_existing_position() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions = vec![Position {
        symbol: "AAPL".to_string(),
        quantity: 100.0,
        avg_entry_price: 150.0,
        current_price: 155.0,
    }];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    let order = rm.evaluate_signal(&signal, &account, &positions, 155.0);

    assert!(order.is_none(), "Should reject buy when position exists");
}

#[test]
fn test_risk_manager_position_sizing() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    // With strength 1.0 and 10% max, order should be ~$10,000
    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 1.0,
    };

    let order = rm
        .evaluate_signal(&signal, &account, &positions, 100.0)
        .unwrap();

    // 10% of $100k = $10k, at $100/share = 100 shares
    assert_eq!(order.quantity, 100.0);
}

#[test]
fn test_risk_manager_scales_by_strength() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    // With strength 0.5, should be half the max position
    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.5,
    };

    let order = rm
        .evaluate_signal(&signal, &account, &positions, 100.0)
        .unwrap();

    // 10% * 0.5 = 5% of $100k = $5k, at $100/share = 50 shares
    assert_eq!(order.quantity, 50.0);
}

#[test]
fn test_risk_manager_respects_daily_limit() {
    let config = RiskConfig {
        max_daily_trades: 2,
        ..default_risk_config()
    };
    let mut rm = RiskManager::new(config);
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    // First two trades should succeed
    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_some()
    );

    let signal2 = Signal::Buy {
        symbol: "MSFT".to_string(),
        strength: 0.8,
    };
    assert!(
        rm.evaluate_signal(&signal2, &account, &positions, 300.0)
            .is_some()
    );

    // Third trade should be rejected
    let signal3 = Signal::Buy {
        symbol: "GOOGL".to_string(),
        strength: 0.8,
    };
    assert!(
        rm.evaluate_signal(&signal3, &account, &positions, 100.0)
            .is_none()
    );
}

#[test]
fn test_risk_manager_reset_daily() {
    let config = RiskConfig {
        max_daily_trades: 1,
        ..default_risk_config()
    };
    let mut rm = RiskManager::new(config);
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    // First trade
    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_some()
    );

    // Should be rejected
    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_none()
    );

    // Reset
    rm.reset_daily();

    // Should work again
    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_some()
    );
}

#[test]
fn test_risk_manager_sell_existing_position() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions = vec![Position {
        symbol: "AAPL".to_string(),
        quantity: 100.0,
        avg_entry_price: 150.0,
        current_price: 160.0,
    }];

    let signal = Signal::Sell {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    let order = rm.evaluate_signal(&signal, &account, &positions, 160.0);

    assert!(order.is_some());
    let order = order.unwrap();
    assert!(matches!(order.side, Side::Sell));
    assert_eq!(order.quantity, 100.0); // Sells entire position
}

#[test]
fn test_risk_manager_rejects_sell_no_position() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Sell {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };

    let order = rm.evaluate_signal(&signal, &account, &positions, 160.0);

    assert!(order.is_none(), "Should reject sell when no position");
}

#[test]
fn test_risk_manager_respects_total_exposure() {
    let config = RiskConfig {
        max_position_pct: 0.50,   // 50% per position
        max_total_exposure: 0.60, // 60% total
        ..default_risk_config()
    };
    let mut rm = RiskManager::new(config);
    let account = Account {
        equity: 100_000.0,
        cash: 50_000.0,
        buying_power: 50_000.0,
    };
    let positions = vec![Position {
        symbol: "AAPL".to_string(),
        quantity: 100.0,
        avg_entry_price: 500.0,
        current_price: 500.0, // $50k position = 50% exposure
    }];

    let signal = Signal::Buy {
        symbol: "MSFT".to_string(),
        strength: 1.0,
    };

    // Should reject because adding more would exceed 60% total exposure
    let order = rm.evaluate_signal(&signal, &account, &positions, 300.0);

    // May be rejected or reduced - depends on implementation
    if let Some(o) = order {
        let new_exposure = 50_000.0 + (o.quantity * 300.0);
        assert!(new_exposure <= 60_000.0, "Should not exceed max exposure");
    }
}

#[test]
fn test_risk_manager_hold_signal() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Hold;
    let order = rm.evaluate_signal(&signal, &account, &positions, 150.0);

    assert!(order.is_none(), "Hold signal should not generate order");
}
