use std::ffi::{c_float, c_int};

#[repr(C)]
pub struct StatsResult {
    pub mean: f64,
    pub variance: f64,
    pub stddev: f64,
}

#[link(name = "cudastats")]
unsafe extern "C" {
    fn compute_stats(data: *const f32, n: c_int, result: *mut StatsResult) -> c_int;
    fn compute_sharpe(
        returns: *const f32,
        n: c_int,
        risk_free_rate: f32,
        sharpe: *mut f64,
    ) -> c_int;
}

#[derive(Debug)]
pub enum ModelError {
    ModelErrorFail,
    ModelErrorInvalidInput,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::ModelErrorFail => write!(f, "Model execution failed"),
            ModelError::ModelErrorInvalidInput => write!(f, "Invalid input data"),
        }
    }
}

impl std::error::Error for ModelError {}

pub fn stats(data: &[f32]) -> Result<StatsResult, ModelError> {
    if data.is_empty() {
        return Err(ModelError::ModelErrorInvalidInput);
    }

    let mut result = StatsResult {
        mean: 0.0,
        variance: 0.0,
        stddev: 0.0,
    };

    let err = unsafe { compute_stats(data.as_ptr(), data.len() as c_int, &mut result) };

    if err != 0 {
        return Err(ModelError::ModelErrorFail);
    }

    Ok(result)
}

pub fn sharpe(returns: &[f32], risk_free_rate: f32) -> Result<f64, ModelError> {
    if returns.is_empty() {
        return Err(ModelError::ModelErrorInvalidInput);
    }

    let mut result = 0.0;

    let err = unsafe {
        compute_sharpe(
            returns.as_ptr(),
            returns.len() as c_int,
            risk_free_rate as c_float,
            &mut result,
        )
    };

    if err != 0 {
        return Err(ModelError::ModelErrorFail);
    }

    Ok(result)
}
