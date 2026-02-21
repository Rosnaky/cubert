use std::time::SystemTime;

use cubert::strategy::Strategy; // Add this
use cubert::strategy::momentum::MomentumStrategy;
use cubert::types::{Bar, Signal}; // Add Bar here

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
    let strategy = MomentumStrategy::new("test_momentum", vec!["AAPL".to_string()], 5, 0.02);

    assert_eq!(strategy.name(), "test_momentum");
    assert_eq!(strategy.symbols(), &["AAPL".to_string()]);
}

#[test]
fn test_momentum_needs_warmup() {
    let mut strategy = MomentumStrategy::new(
        "test",
        vec!["AAPL".to_string()],
        5, // lookback period
        0.02,
    );

    // First few bars should return None (not enough data)
    for i in 0..4 {
        let bar = make_bar("AAPL", 100.0 + i as f64);
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
        "test",
        vec!["AAPL".to_string()],
        5,
        0.02, // 2% threshold
    );

    // Feed rising prices: 100, 101, 102, 103, 104, 105
    for i in 0..6 {
        let price = 100.0 + i as f64;
        let bar = make_bar("AAPL", price);
        let signal = strategy.on_bar(&bar);

        if i >= 5 {
            // After warmup, should generate buy signal (5% gain > 2% threshold)
            match signal {
                Some(Signal::Buy { symbol, strength }) => {
                    assert_eq!(symbol, "AAPL");
                    assert!(strength > 0.0);
                }
                _ => panic!("Expected Buy signal, got {:?}", signal),
            }
        }
    }
}

#[test]
fn test_momentum_generates_sell_signal() {
    let mut strategy = MomentumStrategy::new("test", vec!["AAPL".to_string()], 5, 0.02);

    // Feed falling prices: 110, 108, 106, 104, 102, 100
    for i in 0..6 {
        let price = 110.0 - (i as f64 * 2.0);
        let bar = make_bar("AAPL", price);
        let signal = strategy.on_bar(&bar);

        if i >= 5 {
            // After warmup, should generate sell signal
            match signal {
                Some(Signal::Sell { symbol, strength }) => {
                    assert_eq!(symbol, "AAPL");
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
        "test",
        vec!["AAPL".to_string()],
        5,
        0.10, // 10% threshold - high
    );

    // Feed flat prices
    for _ in 0..10 {
        let bar = make_bar("AAPL", 100.0);
        let signal = strategy.on_bar(&bar);

        // Should be None or Hold (no significant movement)
        if let Some(sig) = signal {
            match sig {
                Signal::Hold => {} // OK
                Signal::Buy { .. } | Signal::Sell { .. } => {
                    panic!("Should not generate Buy/Sell signal for flat prices");
                }
            }
        }
    }
}

#[test]
fn test_momentum_ignores_other_symbols() {
    let mut strategy = MomentumStrategy::new("test", vec!["AAPL".to_string()], 5, 0.02);

    // Feed bars for a different symbol
    let bar = make_bar("MSFT", 100.0);
    let signal = strategy.on_bar(&bar);

    assert!(signal.is_none(), "Should ignore symbols not in strategy");
}

#[test]
fn test_strategy_reset() {
    let mut strategy = MomentumStrategy::new("test", vec!["AAPL".to_string()], 5, 0.02);

    // Feed some data
    for i in 0..10 {
        let bar = make_bar("AAPL", 100.0 + i as f64);
        strategy.on_bar(&bar);
    }

    // Reset
    strategy.reset();

    // After reset, should need warmup again
    let bar = make_bar("AAPL", 150.0);
    let signal = strategy.on_bar(&bar);
    assert!(signal.is_none(), "Should need warmup after reset");
}
