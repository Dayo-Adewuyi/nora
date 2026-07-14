use chew_corpus::{
    align_headings, compare_candidates, normalize_layout_text, AlignmentStatus,
    ClinicalReviewStatus, ComparisonStatus, DoseCandidate, HeadingLevel, HeadingRecord,
};

fn candidate(id: &str, source: &str, text: &str) -> DoseCandidate {
    DoseCandidate {
        schema_version: 1,
        candidate_id: id.into(),
        source_id: source.into(),
        heading_section: Some("2.3".into()),
        physical_pages: vec![1],
        printed_page_labels: vec!["53".into()],
        exact_text: text.into(),
        content_hash: "0".repeat(64),
        signals: vec![],
        comparison_status: ComparisonStatus::NotCompared,
        clinical_review_status: ClinicalReviewStatus::PendingClinicalReview,
    }
}

fn heading(source: &str) -> HeadingRecord {
    HeadingRecord {
        schema_version: 1,
        source_id: source.into(),
        section_number: "2.3".into(),
        exact_title: "FEVER".into(),
        normalized_title: "FEVER".into(),
        level: HeadingLevel::Numbered,
        physical_page: 1,
        printed_page_label: Some("53".into()),
    }
}

#[test]
fn layout_normalization_joins_wrapped_words() {
    assert_eq!(
        normalize_layout_text("para-\ncetamol  5 mg"),
        "PARACETAMOL 5 MG"
    );
}

#[test]
fn clinically_significant_changes_are_content_mismatches() {
    let original = candidate(
        "text",
        "text",
        "Amoxicillin 2.5 mL IM 8 hourly for 5 days weight 5-14 kg REFER; do not repeat",
    );
    for changed in [
        "Amoxicillin 5 mL IM 8 hourly for 5 days weight 5-14 kg REFER; do not repeat",
        "Amoxicillin 2.5 mcg IM 8 hourly for 5 days weight 5-14 kg REFER; do not repeat",
        "Amoxicillin 2.5 mL IV 8 hourly for 5 days weight 5-14 kg REFER; do not repeat",
        "Amoxicillin 2.5 mL IM 12 hourly for 5 days weight 5-14 kg REFER; do not repeat",
        "Amoxicillin 2.5 mL IM 8 hourly for 3 days weight 5-14 kg REFER; do not repeat",
        "Amoxicillin 2.5 mL IM 8 hourly for 5 days weight 15-24 kg REFER; do not repeat",
        "Amoxicillin 2.5 mL IM 8 hourly for 5 days weight 5-14 kg; repeat",
    ] {
        let illustrated = candidate("illustrated", "illustrated", changed);
        assert_eq!(
            compare_candidates(std::slice::from_ref(&original), &[illustrated], &[])[0].status,
            ComparisonStatus::ContentMismatch
        );
    }
}

#[test]
fn duplicate_heading_alignment_is_ambiguous() {
    let alignments = align_headings(&[heading("text")], &[heading("i1"), heading("i2")]);
    assert_eq!(alignments[0].status, AlignmentStatus::Ambiguous);
}
