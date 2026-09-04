use std::time::SystemTime;

use cubert::strategy::Strategy;
use cubert::strategy::mean_reversion::MeanReversionStrategy;
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

fn mean_reversion_config(
    symbols: Vec<String>,
    window: i32,
) -> cubert::strategy::mean_reversion::MeanReversionConfig {
    cubert::strategy::mean_reversion::MeanReversionConfig {
        name: "mean_reversion".to_string(),
        symbols,
        window,
        zscore_entry: 2.0,
        zscore_exit: 0.5,
        min_half_life: 2.0,
        max_half_life: 50.0,
        baseline_vol: 0.02,
        adf_lags: 1,
        recompute_interval: 5,
        max_buffer: 500,
        startup_lookback: "1Hour".to_string(),
        startup_bar_limit: 500,
    }
}

/// Deterministic pseudo-random noise, so the tests do not need a rng crate.
fn noise(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 33) as f64 / (1u64 << 31) as f64) - 1.0
}

/// An Ornstein-Uhlenbeck path: strongly mean reverting, so ADF should reject
/// and the half-life should land inside the configured band.
fn ou_series(n: usize, mean: f64, theta: f64, sigma: f64) -> Vec<f64> {
    let mut seed = 0x5eed_1234_u64;
    let mut price = mean;
    (0..n)
        .map(|_| {
            price += theta * (mean - price) + sigma * noise(&mut seed);
            price
        })
        .collect()
}

#[test]
fn test_mean_reversion_generates_signals_on_stationary_series() {
    let mut strategy =
        MeanReversionStrategy::new(mean_reversion_config(vec!["OU".to_string()], 60));

    let prices = ou_series(400, 100.0, 0.15, 0.8);

    let mut buys = 0;
    let mut sells = 0;
    for price in &prices {
        match strategy.on_bar(&make_bar("OU", *price)) {
            Some(Signal::Buy { strength, .. }) => {
                assert!(
                    (0.0..=1.0).contains(&strength),
                    "strength out of range: {strength}"
                );
                buys += 1;
            }
            Some(Signal::Sell { .. }) => sells += 1,
            _ => {}
        }
    }

    assert!(buys > 0, "expected buy signals on a mean-reverting series");
    assert!(sells > 0, "expected the buys to be followed by exits");
}

#[test]
fn test_mean_reversion_silent_on_trending_series() {
    let mut strategy =
        MeanReversionStrategy::new(mean_reversion_config(vec!["TREND".to_string()], 60));

    // A pure ramp is a unit root: ADF must not reject, so nothing should fire.
    for i in 0..400 {
        let signal = strategy.on_bar(&make_bar("TREND", 100.0 + i as f64 * 0.5));
        assert!(signal.is_none(), "trending series produced {:?}", signal);
    }
}

#[test]
fn test_mean_reversion_needs_warmup() {
    let window = 60;
    let mut strategy =
        MeanReversionStrategy::new(mean_reversion_config(vec!["OU".to_string()], window));

    let prices = ou_series(window as usize, 100.0, 0.15, 0.8);
    for price in &prices {
        let signal = strategy.on_bar(&make_bar("OU", *price));
        assert!(
            signal.is_none(),
            "no signal is possible before {} bars, got {:?}",
            window + 1,
            signal
        );
    }
}

#[test]
fn test_mean_reversion_params_require_baseline_vol() {
    let without_baseline_vol = r#"{"type": "MeanReversion", "window": 60, "zscore_entry": 0.5,
        "zscore_exit": 0.5, "min_half_life": 10.0, "max_half_life": 5000.0, "adf_lags": 1,
        "recompute_interval": 5, "max_buffer": 10000, "startup_lookback": "1Hour",
        "startup_bar_limit": 500}"#;

    let parsed: Result<cubert::strategy::StrategyParams, _> =
        serde_json::from_str(without_baseline_vol);
    assert!(
        parsed.is_err(),
        "baseline_vol is required; params_json rows without it must be backfilled"
    );

    let with_baseline_vol =
        without_baseline_vol.replace(r#""adf_lags""#, r#""baseline_vol": 0.02, "adf_lags""#);
    let params: cubert::strategy::StrategyParams =
        serde_json::from_str(&with_baseline_vol).expect("params carrying baseline_vol must load");

    match params {
        cubert::strategy::StrategyParams::MeanReversion { baseline_vol, .. } => {
            assert_eq!(baseline_vol, 0.02);
        }
        other => panic!("expected MeanReversion, got {:?}", other),
    }

    let round_tripped = serde_json::to_string(&params).unwrap();
    assert!(
        round_tripped.contains("baseline_vol"),
        "baseline_vol must survive a round trip: {round_tripped}"
    );
}

#[test]
fn test_mean_reversion_diagnostics_explain_silence() {
    let mut strategy =
        MeanReversionStrategy::new(mean_reversion_config(vec!["TREND".to_string()], 60));

    for i in 0..200 {
        strategy.on_bar(&make_bar("TREND", 100.0 + i as f64 * 0.5));
    }

    let lines = strategy.diagnostics();
    assert_eq!(lines.len(), 1, "one line per configured symbol");
    assert!(
        lines[0].contains("adf_tau") && lines[0].contains("no entry"),
        "diagnostics should name the failing gate: {}",
        lines[0]
    );

    let mut warming =
        MeanReversionStrategy::new(mean_reversion_config(vec!["TREND".to_string()], 60));
    warming.on_bar(&make_bar("TREND", 100.0));
    assert!(
        warming.diagnostics()[0].contains("warming up, 1/61 bars"),
        "warmup progress should be visible: {}",
        warming.diagnostics()[0]
    );
}
