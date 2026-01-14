use std::f64;

/// Compute Autocorrelation coefficients
pub fn autocorrelation(signal: &[i32], order: usize) -> Vec<f64> {
    let mut r = vec![0.0; order + 1];
    for k in 0..=order {
        let mut sum = 0.0;
        for i in k..signal.len() {
            sum += (signal[i] as f64) * (signal[i - k] as f64);
        }
        r[k] = sum;
    }
    r
}

/// Levinson-Durbin Recursion to find LPC coefficients
/// Returns (coefficients, reflection_coefficients, error)
pub fn levinson_durbin(r: &[f64], order: usize) -> Vec<f64> {
    if r[0].abs() < 1e-9 {
        return vec![0.0; order];
    }

    let mut a = vec![0.0; order + 1];
    let mut e = r[0];

    a[0] = 1.0;

    for k in 1..=order {
        let mut lambda = 0.0;
        for j in 0..k {
            lambda -= a[j] * r[k - j];
        }
        lambda /= e;

        // Update coefficients
        let mut new_a = a.clone();
        for j in 1..k {
            new_a[j] = a[j] + lambda * a[k - j];
        }
        new_a[k] = lambda;
        a = new_a;

        // Update error
        e *= 1.0 - lambda * lambda;
    }

    // Return only the predictor coefficients (excluding a[0] which is 1.0)
    a[1..].to_vec()
}

/// Apply LPC prediction to calculate residuals
pub fn compute_residuals(signal: &[i32], coeffs: &[f64]) -> Vec<i32> {
    let mut residuals = Vec::with_capacity(signal.len());
    let order = coeffs.len();

    for i in 0..signal.len() {
        let mut prediction = 0.0;
        for j in 0..order {
            if i > j {
                prediction -= coeffs[j] * (signal[i - j - 1] as f64);
            }
        }
        
        // Round to nearest integer for integer-to-integer reversibility
        let pred_int = prediction.round() as i32;
        let residual = signal[i] - pred_int;
        residuals.push(residual);
    }
    residuals
}

/// Restore signal from residuals using LPC coefficients
pub fn restore_signal(residuals: &[i32], coeffs: &[f64]) -> Vec<i32> {
    let mut signal = Vec::with_capacity(residuals.len());
    let order = coeffs.len();

    for i in 0..residuals.len() {
        let mut prediction = 0.0;
        for j in 0..order {
            if i > j {
                prediction -= coeffs[j] * (signal[i - j - 1] as f64);
            }
        }
        
        let pred_int = prediction.round() as i32;
        let original = residuals[i] + pred_int;
        signal.push(original);
    }
    signal
}
