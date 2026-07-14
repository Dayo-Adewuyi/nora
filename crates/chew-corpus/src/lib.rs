mod bbox;
mod config;
mod error;
mod manifest;
mod page;
mod toolchain;

pub use bbox::{parse_bbox_layout, BoundingPage, TextBlock, TextLine, Word};
pub use config::{PipelineConfig, SourcePair};
pub use error::CorpusError;
pub use manifest::{Representation, SourceCadre, SourceManifest, VerifiedSource};
pub use page::{build_page_records, ExtractionWarning, PageRecord};
pub use toolchain::{PdfMetadata, PopplerToolchain, RawExtraction};

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
