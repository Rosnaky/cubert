pub mod mean_reversion;
pub mod momentum;

use crate::types::{Bar, Signal};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct StrategySettings {
    pub name: String,
    pub symbols: Vec<String>,
    pub params: StrategyParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StrategyParams {
    Momentum {
        lookback_period: usize,
        threshold: f64,
        startup_lookback: String,
        startup_bar_limit: u32,
    },
    MeanReversion {
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
    },
    Custom {
        params: std::collections::HashMap<String, f64>,
    },
}

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn symbols(&self) -> &[String];
    fn on_bar(&mut self, bar: &Bar) -> Option<Signal>;
    fn reset(&mut self);
    fn startup_config(&self) -> Option<(String, u32)> {
        None
    }
    fn params(&self) -> StrategyParams;
}

pub fn create_strategy(config: &StrategySettings) -> Box<dyn Strategy> {
    match &config.params {
        StrategyParams::Momentum {
            lookback_period,
            threshold,
            startup_lookback,
            startup_bar_limit,
        } => Box::new(momentum::MomentumStrategy::new(
            &config.name,
            config.symbols.clone(),
            *lookback_period,
            *threshold,
            startup_lookback.clone(),
            *startup_bar_limit,
        )),
        StrategyParams::MeanReversion {
            window,
            zscore_entry,
            zscore_exit,
            min_half_life,
            max_half_life,
            adf_lags,
            recompute_interval,
            max_buffer,
            startup_lookback,
            startup_bar_limit,
        } => Box::new(mean_reversion::MeanReversionStrategy::new(
            mean_reversion::MeanReversionConfig {
                name: config.name.clone(),
                symbols: config.symbols.clone(),
                window: *window,
                zscore_entry: *zscore_entry,
                zscore_exit: *zscore_exit,
                min_half_life: *min_half_life,
                max_half_life: *max_half_life,
                adf_lags: *adf_lags,
                recompute_interval: *recompute_interval,
                max_buffer: *max_buffer,
                startup_lookback: startup_lookback.clone(),
                startup_bar_limit: *startup_bar_limit,
            },
        )),
        StrategyParams::Custom { .. } => {
            todo!("Need to implement")
        }
    }
}
