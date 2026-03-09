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
        window: usize,
        std_devs: f64,
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
}

pub fn create_strategy(config: &StrategySettings) -> Box<dyn Strategy> {
    match &config.params {
        StrategyParams::Momentum {
            lookback_period,
            threshold,
            ..
        } => Box::new(momentum::MomentumStrategy::new(
            &config.name,
            config.symbols.clone(),
            *lookback_period,
            *threshold,
        )),
        StrategyParams::MeanReversion {
            window: _,
            std_devs: _,
        } => {
            todo!("Need to implement")
        }
        StrategyParams::Custom { .. } => {
            todo!("Need to implement")
        }
    }
}
