use chew_corpus::{
    detect_dose_like_pages, extract_headings, group_dose_candidates, validate_dose_coverage,
    ClinicalReviewStatus, ComparisonStatus, PageRecord,
};

fn page(number: u32, text: &str) -> PageRecord {
    PageRecord {
        schema_version: 1,
        source_id: "synthetic".into(),
        physical_page: number,
        printed_page_label: Some((52 + number).to_string()),
        width: 595.0,
        height: 842.0,
        layout_text: text.into(),
        blocks: vec![],
        header_candidates: vec![],
        footer_candidates: vec![],
        normalized_content_hash: "0".repeat(64),
        warnings: vec![],
    }
}

#[test]
fn inventories_and_groups_adjacent_dose_pages() {
    let pages = vec![
        page(
            1,
            "2.3 FEVER\nAmoxicillin 125 mg/5 mL: give 2.5 mL 8 hourly for 5 days",
        ),
        page(2, "Weight 5-14 kg\nGive 5 mg/kg IM stat\nAge 2-11 months"),
        page(3, "Discuss the general meaning of dosage with trainees."),
    ];
    let dose_pages = detect_dose_like_pages(&pages);
    let candidates = group_dose_candidates(&pages, &extract_headings(&pages), &dose_pages);

    assert_eq!(dose_pages.len(), 2);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].physical_pages, vec![1, 2]);
    assert!(candidates[0]
        .signals
        .iter()
        .any(|signal| signal.exact_text == "125 mg/5 mL"));
    assert_eq!(
        candidates[0].comparison_status,
        ComparisonStatus::NotCompared
    );
    assert_eq!(
        candidates[0].clinical_review_status,
        ClinicalReviewStatus::PendingClinicalReview
    );
}

#[test]
fn reports_unaccounted_dose_pages_as_blocking() {
    let pages = vec![page(1, "Give 5 mg/kg IM stat")];
    let dose_pages = detect_dose_like_pages(&pages);
    let issues = validate_dose_coverage(&dose_pages, &[], &[]);

    assert_eq!(issues[0].code, "unaccounted_dose_page");
    assert!(issues[0].blocking);
}
