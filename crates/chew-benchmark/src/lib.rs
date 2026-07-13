#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformanceBudget {
    pub minimum_tokens_per_second: f32,
    pub maximum_total_rss_mb: u32,
    pub maximum_shell_rss_mb: u32,
    pub maximum_temperature_celsius: f32,
    pub maximum_warm_latency_ms: u32,
}

impl PerformanceBudget {
    pub const fn competition() -> Self {
        Self {
            minimum_tokens_per_second: 15.0,
            maximum_total_rss_mb: 6_656,
            maximum_shell_rss_mb: 400,
            maximum_temperature_celsius: 85.0,
            maximum_warm_latency_ms: 8_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PerformanceBudget;

    #[test]
    fn competition_budget_preserves_resource_margin() {
        let budget = PerformanceBudget::competition();
        assert_eq!(budget.minimum_tokens_per_second, 15.0);
        assert_eq!(budget.maximum_total_rss_mb, 6_656);
        assert_eq!(budget.maximum_shell_rss_mb, 400);
        assert_eq!(budget.maximum_temperature_celsius, 85.0);
        assert_eq!(budget.maximum_warm_latency_ms, 8_000);
    }
}
