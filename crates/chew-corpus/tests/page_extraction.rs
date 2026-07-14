use std::{io::Cursor, os::unix::fs::PermissionsExt, path::Path};

use chew_corpus::{build_page_records, parse_bbox_layout, PopplerToolchain};

const BBOX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<html><body><doc>
<page width="595" height="842"><flow><block xMin="10" yMin="10" xMax="100" yMax="30"><line xMin="10" yMin="10" xMax="100" yMax="30"><word xMin="10" yMin="10" xMax="40" yMax="30">PAGE</word><word xMin="45" yMin="10" xMax="55" yMax="30">7</word></line></block></flow></page>
<page width="595" height="842"><flow><block xMin="10" yMin="10" xMax="100" yMax="30"><line xMin="10" yMin="10" xMax="100" yMax="30"><word xMin="10" yMin="10" xMax="40" yMax="30">PAGE</word><word xMin="45" yMin="10" xMax="55" yMax="30">8</word></line></block></flow></page>
</doc></body></html>"#;

#[test]
fn builds_page_records_with_physical_and_printed_pages() {
    let bounding = parse_bbox_layout(Cursor::new(BBOX)).unwrap();
    let pages =
        build_page_records("synthetic", "PAGE 7\nBody\n\x0cPAGE 8\nBody\n", bounding).unwrap();

    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].physical_page, 1);
    assert_eq!(pages[0].printed_page_label.as_deref(), Some("7"));
    assert_eq!(pages[1].printed_page_label.as_deref(), Some("8"));
    assert_eq!(pages[0].blocks[0].lines[0].words[0].text, "PAGE");
}

#[test]
fn preserves_poppler_boxes_that_extend_left_of_the_page() {
    let outside_page = BBOX.replacen("xMin=\"10\"", "xMin=\"-0.289818\"", 1);
    let pages = parse_bbox_layout(Cursor::new(outside_page)).unwrap();
    assert_eq!(pages[0].blocks[0].x_min, -0.289818);
}

#[test]
fn rejects_inverted_bounding_boxes() {
    let invalid = BBOX.replacen("xMax=\"100\"", "xMax=\"-1\"", 1);
    assert!(parse_bbox_layout(Cursor::new(invalid)).is_err());
}

#[test]
fn rejects_layout_and_bbox_page_count_mismatch() {
    let bounding = parse_bbox_layout(Cursor::new(BBOX)).unwrap();
    assert!(build_page_records("synthetic", "PAGE 7\n", bounding).is_err());
}

#[test]
fn poppler_extract_runs_layout_and_bbox_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdfinfo = root.join("tests/fixtures/corpus/fake-pdfinfo.sh");
    let pdftotext = root.join("tests/fixtures/corpus/fake-pdftotext.sh");
    for script in [&pdfinfo, &pdftotext] {
        let mut permissions = std::fs::metadata(script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(script, permissions).unwrap();
    }
    let tools = PopplerToolchain::probe(pdfinfo, pdftotext).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let pdf = temp.path().join("source.pdf");
    std::fs::write(&pdf, b"hello world\n").unwrap();

    let extraction = tools.extract(&pdf, temp.path()).unwrap();

    assert_eq!(extraction.layout_pages.len(), 2);
    assert_eq!(extraction.bounding_pages.len(), 2);
}
