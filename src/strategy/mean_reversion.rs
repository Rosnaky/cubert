use std::collections::HashMap;

use crate::{
    model::ffi::{self},
    strategy::{Strategy, StrategyParams},
    types::{Bar, Signal},
};

pub struct MeanReversionStrategy {
    name: String,
    symbols: Vec<String>,
    window: i32,
    zscore_entry: f64,
    zscore_exit: f64,
    min_half_life: f64,
    max_half_life: f64,
    adf_lags: i32,
    recompute_interval: usize,
    max_buffer: usize,
    startup_lookback: String,
    startup_bar_limit: u32,
    price_buffers: HashMap<String, Vec<f64>>,
    bar_counts: HashMap<String, usize>,
    signals: HashMap<String, SymbolState>,
}

struct SymbolState {
    zscore: f64,
    half_life: f64,
    equilibrium: f64,
    volatility: f64,
    theta: f64,
    is_stationary: bool,
    adf_confidence: AdfConfidence,
}

#[derive(Clone, Copy)]
enum AdfConfidence {
    None,
    TenPercent,
    FivePercent,
    OnePercent,
}

impl AdfConfidence {
    fn weight(self) -> f64 {
        match self {
            AdfConfidence::None => 0.0,
            AdfConfidence::TenPercent => 0.5,
            AdfConfidence::FivePercent => 0.75,
            AdfConfidence::OnePercent => 1.0,
        }
    }
}

pub struct MeanReversionConfig {
    pub name: String,
    pub symbols: Vec<String>,
    pub window: i32,
    pub zscore_entry: f64,
    pub zscore_exit: f64,
    pub min_half_life: f64,
    pub max_half_life: f64,
    pub adf_lags: i32,
    pub recompute_interval: usize,
    pub max_buffer: usize,
    pub startup_lookback: String,
    pub startup_bar_limit: u32,
}

impl MeanReversionStrategy {
    pub fn new(config: MeanReversionConfig) -> Self {
        Self {
            name: config.name,
            symbols: config.symbols,
            window: config.window,
            zscore_entry: config.zscore_entry,
            zscore_exit: config.zscore_exit,
            min_half_life: config.min_half_life,
            max_half_life: config.max_half_life,
            adf_lags: config.adf_lags,
            recompute_interval: config.recompute_interval,
            max_buffer: config.max_buffer,
            startup_lookback: config.startup_lookback,
            startup_bar_limit: config.startup_bar_limit,
            price_buffers: HashMap::new(),
            bar_counts: HashMap::new(),
            signals: HashMap::new(),
        }
    }

    fn recompute(&mut self, symbol: &str) {
        let prices = match self.price_buffers.get(symbol) {
            Some(p) => p,
            None => return,
        };

        let zscores = ffi::zscore(prices, self.window);
        let (speeds, equilibria, volatility_sq) = ffi::ou_estimate(prices, self.window);
        let adf_result = ffi::adf(prices, self.adf_lags);

        let last = prices.len() - 1;
        let speed = speeds[last];

        let half_life = if speed > 0.0 && !speed.is_nan() {
            (2.0_f64).ln() / speed
        } else {
            f64::NAN
        };

        let adf_confidence = if adf_result.reject_1pct {
            AdfConfidence::OnePercent
        } else if adf_result.reject_5pct {
            AdfConfidence::FivePercent
        } else if adf_result.reject_10pct {
            AdfConfidence::TenPercent
        } else {
            AdfConfidence::None
        };

        self.signals.insert(
            symbol.to_string(),
            SymbolState {
                zscore: zscores[last],
                half_life,
                equilibrium: equilibria[last],
                volatility: volatility_sq[last].sqrt(),
                theta: speed,
                is_stationary: adf_result.reject_10pct,
                adf_confidence,
            },
        );
    }

    fn generate_signal(&self, symbol: &str, price: f64) -> Option<Signal> {
        let state = self.signals.get(symbol)?;

        if !state.is_stationary {
            return None;
        }

        if state.half_life.is_nan()
            || state.half_life < self.min_half_life
            || state.half_life > self.max_half_life
        {
            return None;
        }

        let z = state.zscore;
        if z.is_nan() {
            return None;
        }

        let baseline_vol = 0.02;
        let vol_scalar = (baseline_vol / state.volatility).clamp(0.25, 1.5);
        let raw_strength = (z.abs() / self.zscore_entry).min(1.5);
        let strength = (raw_strength * state.adf_confidence.weight() * vol_scalar).min(1.0);

        let expected_move = state.theta * (state.equilibrium - price);
        let edge = expected_move / state.volatility;

        if z < -self.zscore_entry {
            Some(Signal::Buy {
                symbol: symbol.to_string(),
                strength,
            })
        } else if edge < self.zscore_exit {
            Some(Signal::Sell {
                symbol: symbol.to_string(),
                strength,
            })
        } else {
            None
        }
    }
}

impl Strategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn symbols(&self) -> &[String] {
        &self.symbols
    }

    fn on_bar(&mut self, bar: &Bar) -> Option<Signal> {
        let buffer = self.price_buffers.entry(bar.symbol.clone()).or_default();
        buffer.push(bar.close);
        if buffer.len() > self.max_buffer {
            buffer.remove(0);
        }

        let count = self.bar_counts.entry(bar.symbol.clone()).or_insert(0);
        *count += 1;

        if (buffer.len() as i32) < self.window {
            return None;
        }

        if (*count).is_multiple_of(self.recompute_interval) {
            self.recompute(&bar.symbol);
        }

        self.generate_signal(&bar.symbol, bar.close)
    }

    fn reset(&mut self) {
        self.price_buffers.clear();
        self.bar_counts.clear();
        self.signals.clear();
    }

    fn startup_config(&self) -> Option<(String, u32)> {
        Some((self.startup_lookback.clone(), self.startup_bar_limit))
    }

    fn params(&self) -> super::StrategyParams {
        StrategyParams::MeanReversion {
            window: self.window,
            zscore_entry: self.zscore_entry,
            zscore_exit: self.zscore_exit,
            min_half_life: self.min_half_life,
            max_half_life: self.max_half_life,
            adf_lags: self.adf_lags,
            recompute_interval: self.recompute_interval,
            max_buffer: self.max_buffer,
            startup_lookback: self.startup_lookback.clone(),
            startup_bar_limit: self.startup_bar_limit,
        }
    }
}
