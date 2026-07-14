mod config;
mod manifest;

pub use config::{PipelineConfig, SourcePair};
pub use manifest::{Representation, SourceCadre, SourceManifest};

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid pipeline configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificationStatus {
    ReferenceOnly,
    InReview,
    Guided,
    Rejected,
}

impl CertificationStatus {
    pub const fn allows_composed_action(self) -> bool {
        matches!(self, Self::Guided)
    }
}

#[cfg(test)]
mod tests {
    use super::CertificationStatus;

    #[test]
    fn only_guided_protocols_can_compose_actions() {
        assert!(!CertificationStatus::ReferenceOnly.allows_composed_action());
        assert!(!CertificationStatus::InReview.allows_composed_action());
        assert!(CertificationStatus::Guided.allows_composed_action());
        assert!(!CertificationStatus::Rejected.allows_composed_action());
    }
}
