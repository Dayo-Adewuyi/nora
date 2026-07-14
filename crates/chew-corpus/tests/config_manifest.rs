use std::path::Path;

use chew_corpus::{PipelineConfig, Representation, SourceCadre, SourceManifest};

#[test]
fn manifest_requires_stable_identity_and_representation() {
    let manifest: SourceManifest = serde_json::from_str(
        r#"{
          "source_id":"chew-2024-text",
          "title":"Synthetic CHEW Source",
          "cadre":"CHEW",
          "edition":"2024",
          "representation":"text",
          "official_url":"https://chprbn.gov.ng/example.pdf",
          "retrieved_at":"2026-07-14",
          "byte_size":12,
          "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "page_count":2,
          "local_path":"tests/fixtures/corpus/example.pdf"
        }"#,
    )
    .unwrap();

    assert_eq!(manifest.source_id, "chew-2024-text");
    assert_eq!(manifest.cadre, SourceCadre::Chew);
    assert_eq!(manifest.representation, Representation::Text);
}

#[test]
fn repository_pipeline_has_three_explicit_pairs() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = PipelineConfig::load(&repo_root, Path::new("corpus/pipeline.json")).unwrap();

    assert_eq!(config.schema_version, 1);
    assert_eq!(config.pairs.len(), 3);
    assert!(config
        .pairs
        .iter()
        .all(|pair| pair.text != pair.illustrated));
}
