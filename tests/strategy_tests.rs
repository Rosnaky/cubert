use std::time::SystemTime;

use cubert::strategy::Strategy;
use cubert::strategy::momentum::MomentumStrategy;
use cubert::types::{Bar, Signal};

fn make_bar(symbol: &str, close: f64) -> Bar {
    Bar {
        symbol: symbol.to_string(),
        timestamp: SystemTime::now(),
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 1000,
    }
}

#[test]
fn test_momentum_strategy_creation() {
    let strategy = MomentumStrategy::new(
        "aggressive_momentum",
        vec!["AAPL".to_string(), "MSFT".to_string()],
        3,
        0.01,
        "5Min".to_string(),
        100,
    );

    assert_eq!(strategy.name(), "aggressive_momentum");
    assert_eq!(
        strategy.symbols(),
        &["AAPL".to_string(), "MSFT".to_string()]
    );
}

#[test]
fn test_momentum_needs_warmup() {
    let mut strategy = MomentumStrategy::new(
        "slow_momentum",
        vec!["GOOGL".to_string()],
        8,
        0.03,
        "1Day".to_string(),
        200,
    );

    for i in 0..7 {
        let bar = make_bar("GOOGL", 200.0 + i as f64);
        let signal = strategy.on_bar(&bar);
        assert!(
            signal.is_none(),
            "Should be None during warmup, got {:?}",
            signal
        );
    }
}

#[test]
fn test_momentum_generates_buy_signal() {
    let mut strategy = MomentumStrategy::new(
        "mid_momentum",
        vec!["TSLA".to_string()],
        5,
        0.02,
        "15Min".to_string(),
        75,
    );

    for i in 0..6 {
        let price = 100.0 + i as f64;
        let bar = make_bar("TSLA", price);
        let signal = strategy.on_bar(&bar);

        if i >= 5 {
            match signal {
                Some(Signal::Buy { symbol, strength }) => {
                    assert_eq!(symbol, "TSLA");
                    assert!(strength > 0.0);
                }
                _ => panic!("Expected Buy signal, got {:?}", signal),
            }
        }
    }
}

#[test]
fn test_momentum_generates_sell_signal() {
    let mut strategy = MomentumStrategy::new(
        "conservative_momentum",
        vec!["NVDA".to_string()],
        5,
        0.02,
        "30Min".to_string(),
        120,
    );

    for i in 0..6 {
        let price = 110.0 - (i as f64 * 2.0);
        let bar = make_bar("NVDA", price);
        let signal = strategy.on_bar(&bar);

        if i >= 5 {
            match signal {
                Some(Signal::Sell { symbol, strength }) => {
                    assert_eq!(symbol, "NVDA");
                    assert!(strength > 0.0);
                }
                _ => panic!("Expected Sell signal, got {:?}", signal),
            }
        }
    }
}

#[test]
fn test_momentum_no_signal_in_range() {
    let mut strategy = MomentumStrategy::new(
        "high_threshold",
        vec!["AMZN".to_string()],
        4,
        0.10,
        "1Hour".to_string(),
        50,
    );

    for _ in 0..10 {
        let bar = make_bar("AMZN", 100.0);
        let signal = strategy.on_bar(&bar);

        if let Some(sig) = signal {
            match sig {
                Signal::Hold => {}
                Signal::Buy { .. } | Signal::Sell { .. } => {
                    panic!("Should not generate Buy/Sell signal for flat prices");
                }
            }
        }
    }
}

#[test]
fn test_momentum_ignores_other_symbols() {
    let mut strategy = MomentumStrategy::new(
        "aapl_only",
        vec!["AAPL".to_string()],
        6,
        0.05,
        "1Day".to_string(),
        30,
    );

    let bar = make_bar("MSFT", 100.0);
    let signal = strategy.on_bar(&bar);
    assert!(signal.is_none(), "Should ignore symbols not in strategy");
}

#[test]
fn test_strategy_reset() {
    let mut strategy = MomentumStrategy::new(
        "reset_test",
        vec!["META".to_string()],
        5,
        0.02,
        "5Min".to_string(),
        60,
    );

    for i in 0..10 {
        let bar = make_bar("META", 100.0 + i as f64);
        strategy.on_bar(&bar);
    }

    strategy.reset();

    let bar = make_bar("META", 150.0);
    let signal = strategy.on_bar(&bar);
    assert!(signal.is_none(), "Should need warmup after reset");
}
