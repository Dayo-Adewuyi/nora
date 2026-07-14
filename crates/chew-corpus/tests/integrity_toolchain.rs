use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use chew_corpus::{CorpusError, PopplerToolchain, Representation, SourceCadre, SourceManifest};

fn manifest(path: PathBuf, sha256: &str) -> SourceManifest {
    SourceManifest {
        source_id: "synthetic-text".into(),
        title: "Synthetic Source".into(),
        cadre: SourceCadre::Chew,
        edition: "2024".into(),
        representation: Representation::Text,
        official_url: "https://chprbn.gov.ng/example.pdf".into(),
        retrieved_at: "2026-07-14".into(),
        byte_size: 12,
        sha256: sha256.into(),
        page_count: 2,
        local_path: path,
    }
}

#[test]
fn source_verification_accepts_matching_size_and_checksum() {
    let repo = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("source.bin"), b"hello world\n").unwrap();
    let source = manifest(
        PathBuf::from("source.bin"),
        "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447",
    );

    let verified = source.verify(repo.path()).unwrap();

    assert_eq!(verified.verified_sha256, source.sha256);
}

#[test]
fn source_verification_rejects_checksum_mismatch() {
    let repo = tempfile::tempdir().unwrap();
    fs::write(repo.path().join("source.bin"), b"hello world\n").unwrap();
    let source = manifest(PathBuf::from("source.bin"), &"0".repeat(64));

    assert!(matches!(
        source.verify(repo.path()),
        Err(CorpusError::ChecksumMismatch { .. })
    ));
}

#[test]
fn poppler_probe_and_pdf_metadata_use_argument_arrays() {
    let temp = tempfile::tempdir().unwrap();
    let pdfinfo = temp.path().join("pdfinfo");
    let pdftotext = temp.path().join("pdftotext");
    fs::write(
        &pdfinfo,
        "#!/bin/sh\nif [ \"$1\" = \"-v\" ]; then echo 'pdfinfo version 26.04.0' >&2; else printf 'Pages: 2\\nEncrypted: no\\nFile size: 12 bytes\\nPDF version: 1.7\\n'; fi\n",
    )
    .unwrap();
    fs::write(
        &pdftotext,
        "#!/bin/sh\necho 'pdftotext version 26.04.0' >&2\n",
    )
    .unwrap();
    fs::set_permissions(&pdfinfo, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&pdftotext, fs::Permissions::from_mode(0o755)).unwrap();

    let tools = PopplerToolchain::probe(pdfinfo, pdftotext).unwrap();
    let metadata = tools.pdf_metadata(temp.path().join("source.pdf")).unwrap();

    assert_eq!(tools.version, "26.04.0");
    assert_eq!(metadata.pages, 2);
    assert!(!metadata.encrypted);
    assert_eq!(metadata.file_size, 12);
    assert_eq!(metadata.pdf_version, "1.7");
}
