use chew_corpus::{
    extract_headings, extract_toc_references, validate_heading_hierarchy,
    validate_heading_references, HeadingLevel, PageRecord,
};

fn page(number: u32, printed: &str, text: &str) -> PageRecord {
    PageRecord {
        schema_version: 1,
        source_id: "synthetic".into(),
        physical_page: number,
        printed_page_label: Some(printed.into()),
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
fn detects_section_headings_and_number_regressions() {
    let pages = vec![
        page(1, "10", "SECTION TWO\n2.3.1 SIMPLE FEVER"),
        page(2, "11", "2.2 EARLIER TOPIC"),
    ];
    let headings = extract_headings(&pages);

    assert_eq!(headings[0].level, HeadingLevel::Section);
    assert_eq!(
        validate_heading_hierarchy(&headings)[0].code,
        "hierarchy_regression"
    );
}

#[test]
fn resolves_numbered_toc_entries_to_printed_pages() {
    let pages = vec![
        page(
            1,
            "3",
            "TABLE OF CONTENTS\n2.3 FEVER 53\n2.3.1 SIMPLE FEVER 54",
        ),
        page(2, "53", "2.3 FEVER\nNATIONAL STANDING ORDERS"),
        page(3, "54", "2.3.1 SIMPLE FEVER\nDetails"),
    ];

    let headings = extract_headings(&pages);
    let references = extract_toc_references(&pages);
    let results = validate_heading_references(&headings, &references);

    assert_eq!(headings.len(), 2);
    assert_eq!(headings[0].section_number, "2.3");
    assert_eq!(headings[1].section_number, "2.3.1");
    assert_eq!(
        results
            .iter()
            .filter(|result| result.issue.is_none())
            .count(),
        2
    );
}

#[test]
fn reports_wrong_toc_page_as_blocking() {
    let pages = vec![
        page(1, "3", "TABLE OF CONTENTS\n2.3 FEVER 99"),
        page(2, "53", "2.3 FEVER\nDetails"),
    ];
    let results =
        validate_heading_references(&extract_headings(&pages), &extract_toc_references(&pages));

    assert_eq!(
        results[0].issue.as_ref().unwrap().code,
        "page_reference_mismatch"
    );
    assert!(results[0].issue.as_ref().unwrap().blocking);
}
