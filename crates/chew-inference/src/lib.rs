#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceLimits {
    pub context_tokens: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
}

impl InferenceLimits {
    pub const fn competition_default() -> Self {
        Self {
            context_tokens: 2_048,
            max_output_tokens: 180,
            temperature: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InferenceLimits;

    #[test]
    fn competition_defaults_cap_context_and_output() {
        let limits = InferenceLimits::competition_default();
        assert_eq!(limits.context_tokens, 2_048);
        assert_eq!(limits.max_output_tokens, 180);
        assert_eq!(limits.temperature, 0.0);
    }
}
