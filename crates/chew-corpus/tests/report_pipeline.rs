use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use chew_corpus::{
    semantic_artifact_sha256, write_json, write_jsonl, CadreReport, PipelineRunner,
    PopplerToolchain, RunReport, RunSourceSummary, ValidationIssue,
};
use serde::Serialize;

#[derive(Serialize)]
struct Line {
    ordinal: u32,
    value: &'static str,
}

fn sample_report() -> RunReport {
    RunReport {
        schema_version: 1,
        generated_at: "2026-07-14T00:00:00Z".into(),
        poppler_version: "26.04.0".into(),
        sources: vec![RunSourceSummary {
            source_id: "synthetic-text".into(),
            sha256: "a".repeat(64),
            pages: 2,
            headings: 1,
            dose_candidates: 1,
        }],
        comparisons: vec![CadreReport {
            cadre: "CHEW".into(),
            heading_alignments: 1,
            heading_issues: 0,
            dose_like_pages: 1,
            dose_pages_accounted: 1,
            dose_pages_unaccounted: 0,
            dose_candidates: 1,
            content_mismatches: 1,
            approved_dose_candidates: 0,
            pending_clinical_review: 1,
            issues: vec![ValidationIssue {
                code: "content_mismatch".into(),
                blocking: true,
                message: "protected dose tokens differ".into(),
            }],
        }],
        blocking_issues: 1,
        semantic_artifact_sha256: String::new(),
    }
}

#[test]
fn stable_writers_are_byte_identical_and_hash_path_order_independently() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let lines = [
        Line {
            ordinal: 1,
            value: "alpha",
        },
        Line {
            ordinal: 2,
            value: "beta",
        },
    ];
    write_jsonl(&first.path().join("lines.jsonl"), &lines).unwrap();
    write_jsonl(&second.path().join("lines.jsonl"), &lines).unwrap();
    write_json(&first.path().join("report.json"), &sample_report()).unwrap();
    write_json(&second.path().join("report.json"), &sample_report()).unwrap();

    let first_lines = fs::read(first.path().join("lines.jsonl")).unwrap();
    let second_lines = fs::read(second.path().join("lines.jsonl")).unwrap();
    let first_report = fs::read(first.path().join("report.json")).unwrap();
    let second_report = fs::read(second.path().join("report.json")).unwrap();
    assert_eq!(first_lines, second_lines);
    assert_eq!(first_report, second_report);
    assert!(first_lines.ends_with(b"\n"));

    let forward = semantic_artifact_sha256(&[
        ("lines.jsonl", first_lines.as_slice()),
        ("report.json", first_report.as_slice()),
    ]);
    let reverse = semantic_artifact_sha256(&[
        ("report.json", first_report.as_slice()),
        ("lines.jsonl", first_lines.as_slice()),
    ]);
    assert_eq!(forward, reverse);
}

#[test]
fn markdown_surfaces_fingerprints_coverage_mismatches_and_review_queue() {
    let markdown = sample_report().to_markdown();
    for expected in [
        "synthetic-text",
        "Source fingerprints",
        "Heading validation",
        "Dose coverage",
        "Content mismatches",
        "Pending clinical review",
        "protected dose tokens differ",
    ] {
        assert!(
            markdown.contains(expected),
            "report omitted {expected}: {markdown}"
        );
    }
}

#[test]
fn verification_checks_both_manifest_files_and_pdf_metadata() {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo.path().join("manifests")).unwrap();
    fs::write(repo.path().join("text.pdf"), b"hello world\n").unwrap();
    fs::write(repo.path().join("illustrated.pdf"), b"hello world\n").unwrap();
    let sha = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";
    for (name, representation, local_path) in [
        ("text", "text", "text.pdf"),
        ("illustrated", "illustrated", "illustrated.pdf"),
    ] {
        fs::write(
            repo.path().join(format!("manifests/{name}.json")),
            format!(
                r#"{{"source_id":"synthetic-{name}","title":"Synthetic","cadre":"CHEW","edition":"2024","representation":"{representation}","official_url":"https://chprbn.gov.ng/{local_path}","retrieved_at":"2026-07-14","byte_size":12,"sha256":"{sha}","page_count":2,"local_path":"{local_path}"}}"#
            ),
        )
        .unwrap();
    }
    fs::write(
        repo.path().join("pipeline.json"),
        r#"{"schema_version":1,"derived_path":"derived","pairs":[{"cadre":"CHEW","text":"manifests/text.json","illustrated":"manifests/illustrated.json"}]}"#,
    )
    .unwrap();
    let pdfinfo = repo.path().join("pdfinfo");
    let pdftotext = repo.path().join("pdftotext");
    fs::write(&pdfinfo, "#!/bin/sh\nif [ \"$1\" = \"-v\" ]; then echo 'pdfinfo version 26.04.0' >&2; else printf 'Pages: 2\\nEncrypted: no\\nFile size: 12 bytes\\nPDF version: 1.7\\n'; fi\n").unwrap();
    fs::write(
        &pdftotext,
        "#!/bin/sh\necho 'pdftotext version 26.04.0' >&2\n",
    )
    .unwrap();
    fs::set_permissions(&pdfinfo, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&pdftotext, fs::Permissions::from_mode(0o755)).unwrap();
    let tools = PopplerToolchain::probe(pdfinfo, pdftotext).unwrap();

    let report = PipelineRunner::from_toolchain(repo.path().to_path_buf(), tools)
        .verify(&PathBuf::from("pipeline.json"))
        .unwrap();

    assert_eq!(report.sources.len(), 2);
    assert_eq!(report.poppler_version, "26.04.0");
}
