
use crate::strategy::Strategy;
use crate::strategy::Bar;
use crate::types::Signal;
use std::collections::HashMap;

pub struct MomentumStrategy {
    name: String,
    symbols: Vec<String>,
    lookback_period: usize,
    threshold: f64,
    history: HashMap<String, Vec<f64>>,
}

impl MomentumStrategy {
    pub fn new(
        name: &str,
        symbols: Vec<String>,
        lookback_period: usize,
        threshold: f64,
    ) -> Self {
        let mut history = HashMap::new();
        for symbol in &symbols {
            history.insert(symbol.clone(), Vec::with_capacity(lookback_period + 1));
        }

        Self {
            name: name.to_string(),
            symbols,
            lookback_period,
            threshold,
            history,
        }
    }

    fn calculate_momentum(&self, prices: &[f64]) -> Option<f64> {
        if prices.len() < self.lookback_period {
            return None;
        }

        let current = *prices.last()?;
        let past = prices[prices.len() - self.lookback_period];

        if past == 0.0 {
            return None;
        }

        Some((current-past)/past)
    }
}

impl Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    fn symbols (&self) -> &[String] {
        &self.symbols
    }
    fn on_bar(&mut self, bar: &Bar) -> Option<Signal> {
        {
            let history = self.history.get_mut(&bar.symbol)?;

            history.push(bar.close);

            if history.len() > self.lookback_period + 10 {
                history.drain(0..10);
            }
        }

        let history = self.history.get(&bar.symbol)?;
        let momentum = self.calculate_momentum(history)?;

        if momentum > self.threshold {
            Some(Signal::Buy {
                symbol: bar.symbol.clone(),
                strength: momentum.min(1.0),
            })
        }
        else if momentum < -self.threshold {
            Some(Signal::Sell {
                symbol: bar.symbol.clone(),
                strength: momentum.abs().min(1.0),
            })
        }
        else {
            None
        }
    }

    fn reset(&mut self) {
        for history in self.history.values_mut() {
            history.clear();
        }
    }
}