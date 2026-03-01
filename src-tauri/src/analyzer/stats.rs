/// Reusable statistical functions for dashboard analytics.

/// Arithmetic mean. Returns 0.0 if the slice is empty.
pub fn moyenne(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Percentile with linear interpolation. `p` is in [0, 100].
/// Returns 0.0 if the slice is empty.
pub fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    // Rank (0-based fractional index)
    let rank = p / 100.0 * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let frac = rank - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Population standard deviation. Returns 0.0 if the slice is empty.
pub fn ecart_type(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = moyenne(values);
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- moyenne ---

    #[test]
    fn test_moyenne_empty() {
        assert_eq!(moyenne(&[]), 0.0);
    }

    #[test]
    fn test_moyenne_single() {
        assert_eq!(moyenne(&[5.0]), 5.0);
    }

    #[test]
    fn test_moyenne_known() {
        // (2 + 4 + 6) / 3 = 4.0
        assert!((moyenne(&[2.0, 4.0, 6.0]) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_moyenne_decimals() {
        // (1.5 + 2.5 + 3.0) / 3 = 7.0/3 ≈ 2.3333
        let result = moyenne(&[1.5, 2.5, 3.0]);
        assert!((result - 7.0 / 3.0).abs() < 1e-10);
    }

    // --- percentile ---

    #[test]
    fn test_percentile_empty() {
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    #[test]
    fn test_percentile_single() {
        assert_eq!(percentile(&[42.0], 50.0), 42.0);
        assert_eq!(percentile(&[42.0], 90.0), 42.0);
    }

    #[test]
    fn test_percentile_median_odd() {
        // Sorted: [1, 2, 3, 4, 5]. Median (p50) = 3.0
        assert!((percentile(&[3.0, 1.0, 5.0, 2.0, 4.0], 50.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_median_even() {
        // Sorted: [1, 2, 3, 4]. p50 → rank = 0.5 * 3 = 1.5 → lerp(2, 3, 0.5) = 2.5
        assert!((percentile(&[4.0, 1.0, 3.0, 2.0], 50.0) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_p90() {
        // Sorted: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        // rank = 0.9 * 9 = 8.1 → lerp(9, 10, 0.1) = 9.1
        let vals: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        assert!((percentile(&vals, 90.0) - 9.1).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_p0_and_p100() {
        let vals = vec![10.0, 20.0, 30.0];
        assert!((percentile(&vals, 0.0) - 10.0).abs() < 1e-10);
        assert!((percentile(&vals, 100.0) - 30.0).abs() < 1e-10);
    }

    // --- ecart_type ---

    #[test]
    fn test_ecart_type_empty() {
        assert_eq!(ecart_type(&[]), 0.0);
    }

    #[test]
    fn test_ecart_type_single() {
        assert_eq!(ecart_type(&[5.0]), 0.0);
    }

    #[test]
    fn test_ecart_type_uniform() {
        // All same value → std dev = 0
        assert_eq!(ecart_type(&[3.0, 3.0, 3.0]), 0.0);
    }

    #[test]
    fn test_ecart_type_known() {
        // [2, 4, 4, 4, 5, 5, 7, 9] → mean=5, pop std dev=2.0
        let vals = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((ecart_type(&vals) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecart_type_two_values() {
        // [0, 10] → mean=5, variance=25, std dev=5
        assert!((ecart_type(&[0.0, 10.0]) - 5.0).abs() < 1e-10);
    }
}
