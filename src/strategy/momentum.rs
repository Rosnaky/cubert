use std::collections::HashMap;

use crate::strategy::Bar;
use crate::strategy::Strategy;
use crate::strategy::StrategyParams;
use crate::types::Signal;

pub struct MomentumStrategy {
    name: String,
    symbols: Vec<String>,
    lookback_period: usize,
    threshold: f64,
    startup_lookback: String,
    startup_bar_limit: u32,
    price_history: HashMap<String, Vec<f64>>,
}

impl MomentumStrategy {
    pub fn new(
        name: &str,
        symbols: Vec<String>,
        lookback_period: usize,
        threshold: f64,
        startup_lookback: String,
        startup_bar_limit: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            symbols,
            lookback_period,
            threshold,
            startup_lookback,
            startup_bar_limit,
            price_history: HashMap::new(),
        }
    }

    fn calculate_momentum(prices: &[f64], lookback_period: usize) -> Option<f64> {
        if prices.len() < lookback_period {
            return None;
        }

        let current = *prices.last()?;
        let past = prices[prices.len() - lookback_period];

        if past == 0.0 {
            return None;
        }

        Some((current - past) / past)
    }
}

impl Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
    fn on_bar(&mut self, bar: &Bar) -> Option<Signal> {
        let history = self.price_history.entry(bar.symbol.clone()).or_default();

        history.push(bar.close);

        if history.len() > self.lookback_period + 10 {
            history.drain(0..10);
        }

        let momentum = MomentumStrategy::calculate_momentum(history, self.lookback_period)?;

        if momentum > self.threshold {
            Some(Signal::Buy {
                symbol: bar.symbol.clone(),
                strength: momentum.min(1.0),
            })
        } else if momentum < -self.threshold {
            Some(Signal::Sell {
                symbol: bar.symbol.clone(),
                strength: momentum.abs().min(1.0),
            })
        } else {
            None
        }
    }

    fn reset(&mut self) {
        for history in self.price_history.values_mut() {
            history.clear();
        }
    }

    fn startup_config(&self) -> Option<(String, u32)> {
        Some((self.startup_lookback.clone(), self.startup_bar_limit))
    }

    fn params(&self) -> StrategyParams {
        StrategyParams::Momentum {
            lookback_period: self.lookback_period,
            threshold: self.threshold,
            startup_lookback: self.startup_lookback.clone(),
            startup_bar_limit: self.startup_bar_limit,
        }
    }
}
