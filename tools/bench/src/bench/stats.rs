//! The few statistics the benchmark reports: median, percentiles and spread.

/// The middle value (the mean of the two middle ones for an even count); 0 for no values.
pub fn median(values: &[f64]) -> f64 {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// The `p`-th percentile (0..=100) of values sorted ascending, interpolating between neighbours.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    match sorted.len() {
        0 => 0.0,
        1 => sorted[0],
        n => {
            let pos = p.clamp(0.0, 100.0) / 100.0 * (n - 1) as f64;
            let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
            sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
        }
    }
}

pub fn min_max(values: &[f64]) -> (f64, f64) {
    let mut it = values.iter().copied().filter(|x| x.is_finite());
    let Some(first) = it.next() else {
        return (0.0, 0.0);
    };
    it.fold((first, first), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

/// Half the range as a share of the median: the "±" shown next to a result.
pub fn spread_pct(median: f64, min: f64, max: f64) -> f64 {
    if median <= 0.0 { 0.0 } else { (max - min) / 2.0 / median * 100.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_odd_and_even_counts() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert_eq!(median(&[]), 0.0);
        assert_eq!(median(&[f64::NAN, 5.0]), 5.0);
    }

    #[test]
    fn percentiles_interpolate() {
        let v = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(percentile(&v, 0.0), 10.0);
        assert_eq!(percentile(&v, 50.0), 30.0);
        assert_eq!(percentile(&v, 100.0), 50.0);
        assert_eq!(percentile(&v, 25.0), 20.0);
        assert!((percentile(&v, 90.0) - 46.0).abs() < 1e-9);
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    #[test]
    fn spread_is_half_the_range_over_the_median() {
        assert_eq!(min_max(&[3.0, 1.0, 2.0]), (1.0, 3.0));
        assert!((spread_pct(100.0, 90.0, 110.0) - 10.0).abs() < 1e-9);
        assert_eq!(spread_pct(0.0, 0.0, 1.0), 0.0);
    }
}
