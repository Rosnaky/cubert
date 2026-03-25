use cubert::config::RiskConfig;
use cubert::risk::RiskManager;
use cubert::types::{Account, Position, Side, Signal};

fn default_risk_config() -> RiskConfig {
    RiskConfig {
        max_position_pct: 0.10,
        max_drawdown_pct: 0.05,
        max_daily_trades: 10,
        max_total_exposure: 0.80,
        max_loss_per_trade: 0.02,
    }
}

fn default_account() -> Account {
    Account {
        id: "test_default".to_string(),
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

    assert!(order.is_some());
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
    assert!(order.is_none());
}

#[test]
fn test_risk_manager_position_sizing() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 1.0,
    };

    let order = rm
        .evaluate_signal(&signal, &account, &positions, 100.0)
        .unwrap();
    assert_eq!(order.quantity, 100.0);
}

#[test]
fn test_risk_manager_scales_by_strength() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.5,
    };

    let order = rm
        .evaluate_signal(&signal, &account, &positions, 100.0)
        .unwrap();
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

    let signal1 = Signal::Buy {
        symbol: "AAPL".to_string(),
        strength: 0.8,
    };
    let signal2 = Signal::Buy {
        symbol: "MSFT".to_string(),
        strength: 0.8,
    };
    let signal3 = Signal::Buy {
        symbol: "GOOGL".to_string(),
        strength: 0.8,
    };

    assert!(
        rm.evaluate_signal(&signal1, &account, &positions, 150.0)
            .is_some()
    );
    assert!(
        rm.evaluate_signal(&signal2, &account, &positions, 300.0)
            .is_some()
    );
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

    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_some()
    );
    assert!(
        rm.evaluate_signal(&signal, &account, &positions, 150.0)
            .is_none()
    );

    rm.reset_daily();

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
    assert_eq!(order.quantity, 100.0);
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
    assert!(order.is_none());
}

#[test]
fn test_risk_manager_hold_signal() {
    let mut rm = RiskManager::new(default_risk_config());
    let account = default_account();
    let positions: Vec<Position> = vec![];

    let signal = Signal::Hold;
    let order = rm.evaluate_signal(&signal, &account, &positions, 150.0);

    assert!(order.is_none());
}
