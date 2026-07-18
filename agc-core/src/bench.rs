//! Percentile math for latency reporting, used by `agc-cli bench ingest`
//! to check the "SLA: p99 ingest latency < 10ms for 1K spans/s" item
//! from `ROADMAP.md`'s v1.0.0 milestone.

/// Nearest-rank percentile over an already-ascending-sorted slice.
/// `pct` must be in `[0, 100]`. Empty input returns `0`.
pub fn percentile(sorted_ascending: &[u64], pct: f64) -> u64 {
    if sorted_ascending.is_empty() {
        return 0;
    }
    let rank = ((pct / 100.0) * sorted_ascending.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_ascending.len() - 1);
    sorted_ascending[index]
}

#[derive(Debug, Clone)]
pub struct LatencyReport {
    pub count: usize,
    pub errors: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

impl LatencyReport {
    /// `sorted_ascending_us` must already be sorted ascending -- the
    /// caller (a load generator collecting one latency per request) sorts
    /// once after the run, keeping this constructor itself a simple,
    /// allocation-free pass over the data.
    pub fn from_sorted_micros(sorted_ascending_us: &[u64], errors: u64) -> Self {
        Self {
            count: sorted_ascending_us.len(),
            errors,
            p50_us: percentile(sorted_ascending_us, 50.0),
            p95_us: percentile(sorted_ascending_us, 95.0),
            p99_us: percentile(sorted_ascending_us, 99.0),
            max_us: sorted_ascending_us.last().copied().unwrap_or(0),
        }
    }

    pub fn p99_ms(&self) -> f64 {
        self.p99_us as f64 / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_of_ten_ascending_values_matches_nearest_rank() {
        let values: Vec<u64> = (1..=10).collect();
        assert_eq!(percentile(&values, 50.0), 5);
        assert_eq!(percentile(&values, 95.0), 10);
        assert_eq!(percentile(&values, 99.0), 10);
        assert_eq!(percentile(&values, 100.0), 10);
    }

    #[test]
    fn percentile_of_empty_slice_is_zero() {
        assert_eq!(percentile(&[], 99.0), 0);
    }

    #[test]
    fn percentile_of_single_element_returns_that_element_for_any_percentile() {
        assert_eq!(percentile(&[42], 1.0), 42);
        assert_eq!(percentile(&[42], 99.0), 42);
    }

    #[test]
    fn percentile_of_a_thousand_values_matches_a_hand_verified_p99() {
        // Nearest-rank p99 of 1..=1000 is rank ceil(0.99*1000)=990 -> value 990.
        let values: Vec<u64> = (1..=1000).collect();
        assert_eq!(percentile(&values, 99.0), 990);
    }

    #[test]
    fn latency_report_computes_count_max_and_errors_correctly() {
        let values: Vec<u64> = vec![100, 200, 300, 400, 500];
        let report = LatencyReport::from_sorted_micros(&values, 3);
        assert_eq!(report.count, 5);
        assert_eq!(report.errors, 3);
        assert_eq!(report.max_us, 500);
        assert_eq!(report.p50_us, 300);
    }

    #[test]
    fn p99_ms_converts_microseconds_to_milliseconds() {
        let report = LatencyReport { count: 1, errors: 0, p50_us: 0, p95_us: 0, p99_us: 8_500, max_us: 8_500 };
        assert!((report.p99_ms() - 8.5).abs() < 1e-9);
    }
}
