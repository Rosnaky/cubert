use std::collections::HashMap;

use crate::{
    model::ffi,
    strategy::{Strategy, StrategyParams},
    types::{Bar, Signal},
};

const VOL_SCALAR_MIN: f64 = 0.25;
const VOL_SCALAR_MAX: f64 = 1.5;

pub struct MeanReversionStrategy {
    name: String,
    symbols: Vec<String>,
    window: i32,
    zscore_entry: f64,
    zscore_exit: f64,
    min_half_life: f64,
    max_half_life: f64,
    baseline_vol: f64,
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
    prev_zscore: f64,
    half_life: f64,
    equilibrium: f64,
    volatility: f64,
    adf_tau: f64,
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

    fn is_stationary(self) -> bool {
        !matches!(self, AdfConfidence::None)
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
    pub baseline_vol: f64,
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
            baseline_vol: config.baseline_vol,
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

    fn min_bars(&self) -> usize {
        self.window.max(1) as usize + 1
    }

    fn exit_threshold(&self) -> f64 {
        self.zscore_exit.min(self.zscore_entry)
    }

    fn regime_block(&self, state: &SymbolState) -> Option<String> {
        if !state.adf_confidence.is_stationary() {
            return Some(format!("adf tau {:.2} not below -2.57", state.adf_tau));
        }
        if !state.half_life.is_finite() {
            return Some("half-life undefined, series is not mean reverting".to_string());
        }
        if state.half_life < self.min_half_life {
            return Some(format!(
                "half-life {:.1} below min {:.1}",
                state.half_life, self.min_half_life
            ));
        }
        if state.half_life > self.max_half_life {
            return Some(format!(
                "half-life {:.1} above max {:.1}",
                state.half_life, self.max_half_life
            ));
        }
        None
    }

    fn regime_ok(&self, state: &SymbolState) -> bool {
        self.regime_block(state).is_none()
    }

    fn entry_check(&self, state: &SymbolState, price: f64) -> Result<f64, String> {
        if let Some(reason) = self.regime_block(state) {
            return Err(reason);
        }
        if state.zscore.is_nan() {
            return Err("z-score undefined".to_string());
        }
        if state.zscore >= -self.zscore_entry {
            return Err(format!(
                "z {:.2} not below entry {:.2}",
                state.zscore, -self.zscore_entry
            ));
        }
        if state.equilibrium.is_nan() || price >= state.equilibrium {
            return Err(format!(
                "price {:.2} not below equilibrium {:.2}",
                price, state.equilibrium
            ));
        }
        if !state.volatility.is_finite() || state.volatility <= 0.0 || price <= 0.0 {
            return Err(format!("volatility {:.4} unusable", state.volatility));
        }

        let fractional_vol = state.volatility / price;
        let vol_scalar = if self.baseline_vol > 0.0 {
            (self.baseline_vol / fractional_vol).clamp(VOL_SCALAR_MIN, VOL_SCALAR_MAX)
        } else {
            1.0
        };
        let raw_strength = (state.zscore.abs() / self.zscore_entry).min(1.5);
        let strength = (raw_strength * state.adf_confidence.weight() * vol_scalar).min(1.0);

        if strength <= 0.0 {
            return Err("strength collapsed to zero".to_string());
        }

        Ok(strength)
    }

    fn recompute(&mut self, symbol: &str) {
        let prices = match self.price_buffers.get(symbol) {
            Some(p) if p.len() >= self.min_bars() => p,
            _ => return,
        };

        let zscores = ffi::zscore(prices, self.window);
        let (speeds, equilibria, volatility_sq) = ffi::ou_estimate(prices, self.window);
        let adf_result = ffi::adf(prices, self.adf_lags);

        let last = prices.len() - 1;
        let ou_last = last - 1;

        let speed = speeds[ou_last];

        let half_life = if speed > 0.0 {
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

        let prev_zscore = self
            .signals
            .get(symbol)
            .map(|s| s.zscore)
            .unwrap_or(f64::NAN);

        self.signals.insert(
            symbol.to_string(),
            SymbolState {
                zscore: zscores[last],
                prev_zscore,
                half_life,
                equilibrium: equilibria[ou_last],
                volatility: volatility_sq[ou_last].sqrt(),
                adf_tau: adf_result.tau,
                adf_confidence,
            },
        );
    }

    fn generate_signal(&self, symbol: &str, price: f64) -> Option<Signal> {
        let state = self.signals.get(symbol)?;
        let exit_threshold = self.exit_threshold();

        let was_stretched = state.prev_zscore < -exit_threshold;
        let reverted = state.zscore.is_nan() || state.zscore >= -exit_threshold;
        if was_stretched && (reverted || !self.regime_ok(state)) {
            return Some(Signal::Sell {
                symbol: symbol.to_string(),
                strength: 1.0,
            });
        }

        let strength = self.entry_check(state, price).ok()?;

        Some(Signal::Buy {
            symbol: symbol.to_string(),
            strength,
        })
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
        let buffered = buffer.len();

        let count = self.bar_counts.entry(bar.symbol.clone()).or_insert(0);
        *count += 1;
        let count = *count;

        if buffered < self.min_bars() {
            return None;
        }

        if !count.is_multiple_of(self.recompute_interval.max(1)) {
            return None;
        }

        self.recompute(&bar.symbol);
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

    fn diagnostics(&self) -> Vec<String> {
        let mut out = Vec::new();

        for symbol in &self.symbols {
            let buffer = self.price_buffers.get(symbol);
            let buffered = buffer.map(|b| b.len()).unwrap_or(0);

            if buffered < self.min_bars() {
                out.push(format!(
                    "{}: warming up, {}/{} bars",
                    symbol,
                    buffered,
                    self.min_bars()
                ));
                continue;
            }

            let state = match self.signals.get(symbol) {
                Some(s) => s,
                None => {
                    out.push(format!("{}: {} bars, no estimate yet", symbol, buffered));
                    continue;
                }
            };

            let price = buffer.and_then(|b| b.last().copied()).unwrap_or(f64::NAN);
            let verdict = match self.entry_check(state, price) {
                Ok(strength) => format!("BUY eligible, strength {:.2}", strength),
                Err(reason) => format!("no entry: {}", reason),
            };

            out.push(format!(
                "{}: {} bars, z={:.2} half_life={:.1} adf_tau={:.2} vol={:.4} eq={:.2} px={:.2} -> {}",
                symbol,
                buffered,
                state.zscore,
                state.half_life,
                state.adf_tau,
                state.volatility,
                state.equilibrium,
                price,
                verdict
            ));
        }

        out
    }

    fn params(&self) -> super::StrategyParams {
        StrategyParams::MeanReversion {
            window: self.window,
            zscore_entry: self.zscore_entry,
            zscore_exit: self.zscore_exit,
            min_half_life: self.min_half_life,
            max_half_life: self.max_half_life,
            baseline_vol: self.baseline_vol,
            adf_lags: self.adf_lags,
            recompute_interval: self.recompute_interval,
            max_buffer: self.max_buffer,
            startup_lookback: self.startup_lookback.clone(),
            startup_bar_limit: self.startup_bar_limit,
        }
    }
}
