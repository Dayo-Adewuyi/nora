use chew_corpus::{
    apply_reviews, ClinicalReviewStatus, ComparisonStatus, DoseCandidate, DoseReviewRecord,
    ReviewDecision,
};

fn candidate() -> DoseCandidate {
    DoseCandidate {
        schema_version: 1,
        candidate_id: "dose-text-1-abc".into(),
        source_id: "chew-2024-text".into(),
        heading_section: Some("2.3".into()),
        physical_pages: vec![53],
        printed_page_labels: vec!["53".into()],
        exact_text: "Amoxicillin 2.5 mL".into(),
        content_hash: "a".repeat(64),
        signals: vec![],
        comparison_status: ComparisonStatus::Match,
        clinical_review_status: ClinicalReviewStatus::PendingClinicalReview,
    }
}

fn review(hash: &str, decision: ReviewDecision) -> DoseReviewRecord {
    DoseReviewRecord {
        schema_version: 1,
        candidate_id: "dose-text-1-abc".into(),
        content_hash: hash.into(),
        text_source_id: "chew-2024-text".into(),
        illustrated_source_id: "chew-2024-illustrated".into(),
        reviewer: "Clinical Lead".into(),
        reviewed_at: "2026-07-14".into(),
        decision,
        notes: "Checked against both editions".into(),
    }
}

#[test]
fn exact_hash_review_approves_but_no_review_remains_pending() {
    assert_eq!(
        apply_reviews(&[candidate()], &[]).candidates[0].status,
        ClinicalReviewStatus::PendingClinicalReview
    );
    assert_eq!(
        apply_reviews(
            &[candidate()],
            &[review(&"a".repeat(64), ReviewDecision::Approved)]
        )
        .candidates[0]
            .status,
        ClinicalReviewStatus::Approved
    );
}

#[test]
fn changed_hash_is_stale_and_conflicting_records_are_blocking() {
    let stale = apply_reviews(
        &[candidate()],
        &[review(&"b".repeat(64), ReviewDecision::Approved)],
    );
    assert_eq!(stale.candidates[0].status, ClinicalReviewStatus::Stale);
    assert!(stale.issues[0].blocking);

    let conflict = apply_reviews(
        &[candidate()],
        &[
            review(&"a".repeat(64), ReviewDecision::Approved),
            review(&"a".repeat(64), ReviewDecision::Rejected),
        ],
    );
    assert_eq!(
        conflict.candidates[0].status,
        ClinicalReviewStatus::Conflict
    );
}
