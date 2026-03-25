#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/gem_bindings.rs"));

pub fn zscore(prices: &[f64], window: i32) -> Vec<f64> {
    let mut out = vec![0.0; prices.len()];
    unsafe {
        rolling_zscore_f64(
            prices.as_ptr(),
            out.as_mut_ptr(),
            prices.len() as i32,
            window,
        );
    }
    out
}

pub fn zscore_f32(prices: &[f32], window: i32) -> Vec<f32> {
    let mut out = vec![0.0; prices.len()];
    unsafe {
        rolling_zscore_f32(
            prices.as_ptr(),
            out.as_mut_ptr(),
            prices.len() as i32,
            window,
        );
    }
    out
}

pub fn ou_estimate(prices: &[f64], window: i32) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = prices.len();
    let mut speed = vec![0.0; n];
    let mut equilibrium = vec![0.0; n];
    let mut volatility_sq = vec![0.0; n];
    unsafe {
        ou_estimation_f64(
            prices.as_ptr(),
            n as i32,
            speed.as_mut_ptr(),
            equilibrium.as_mut_ptr(),
            volatility_sq.as_mut_ptr(),
            window,
        );
    }
    (speed, equilibrium, volatility_sq)
}

pub fn ou_estimate_f32(prices: &[f32], window: i32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = prices.len();
    let mut speed = vec![0.0; n];
    let mut equilibrium = vec![0.0; n];
    let mut volatility_sq = vec![0.0; n];
    unsafe {
        ou_estimation_f32(
            prices.as_ptr(),
            n as i32,
            speed.as_mut_ptr(),
            equilibrium.as_mut_ptr(),
            volatility_sq.as_mut_ptr(),
            window,
        );
    }
    (speed, equilibrium, volatility_sq)
}

pub fn adf(prices: &[f64], lags: i32) -> ADFResult_f64 {
    let mut result = unsafe { std::mem::zeroed::<ADFResult_f64>() };
    unsafe {
        adf_test_f64(prices.as_ptr(), prices.len() as i32, lags, &mut result);
    }
    result
}

pub fn adf_f32(prices: &[f32], lags: i32) -> ADFResult_f32 {
    let mut result = unsafe { std::mem::zeroed::<ADFResult_f32>() };
    unsafe {
        adf_test_f32(prices.as_ptr(), prices.len() as i32, lags, &mut result);
    }
    result
}
