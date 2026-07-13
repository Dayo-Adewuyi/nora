# Corpus Extraction and Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deterministic Rust and Poppler pipeline that extracts all six 2024 Standing Orders PDFs with layout provenance, validates headings and page references, inventories and compares every dose-like table, and leaves every dosing table pending doctor review.

**Architecture:** Extend `chew-corpus` with focused modules and a `chew-corpus-pipeline` CLI. The CLI verifies tracked source manifests, invokes fixed Poppler commands, converts layout and bounding-box output into stable JSONL records, aligns text and illustrated editions, emits comparison reports, and validates hash-bound clinical review records. Generated artifacts are ignored; source metadata, configuration, tests, and review decisions are tracked.

**Tech Stack:** Rust 1.88+, Serde/serde_json, clap 4, quick-xml, regex, sha2, unicode-normalization, tempfile, Poppler 26.04.0 (`pdfinfo`, `pdftotext`), Cargo test/clippy/fmt.

## Global Constraints

- The pipeline is fully offline and must never fetch source files at runtime.
- The six raw PDFs under `corpus/raw/` are immutable inputs and remain Git-ignored.
- Text editions are extraction and citation authority; illustrated editions are comparison sources.
- Physical PDF page numbers and visible printed page labels are distinct fields.
- A missing text layer, page, heading reference, or dose-page accounting record is blocking.
- Comparison may normalize layout differences but must preserve numbers, units, drugs, routes, frequencies, durations, age bands, weight bands, negation, referral language, and contraindications.
- Every dose-table candidate starts `pending_clinical_review`; automated matches never grant approval.
- CHO records may provide comparison/referral context but cannot authorize CHEW or JCHEW actions.
- Generated JSON/JSONL uses schema version `1`, stable struct field order, repository-relative paths, and deterministic source ordering.
- A fixed `SOURCE_DATE_EPOCH` must make two equivalent runs produce identical semantic artifact hashes.
- Behavioral code is implemented test-first.

---

## Repository Map for This Feature

```text
Cargo.toml                                      # shared dependency versions
crates/chew-corpus/Cargo.toml                   # corpus crate dependencies and binary
crates/chew-corpus/src/lib.rs                   # public modules and certification contract
crates/chew-corpus/src/main.rs                  # clap CLI only
crates/chew-corpus/src/config.rs                # pipeline configuration and source pairing
crates/chew-corpus/src/manifest.rs              # source metadata and file-integrity checks
crates/chew-corpus/src/toolchain.rs             # pdfinfo/pdftotext process boundary
crates/chew-corpus/src/bbox.rs                  # Poppler XHTML bounding-box parser
crates/chew-corpus/src/page.rs                  # page records and printed-page labels
crates/chew-corpus/src/heading.rs               # headings, TOC references, hierarchy checks
crates/chew-corpus/src/dose.rs                  # dose-page superset and candidate grouping
crates/chew-corpus/src/comparison.rs            # edition alignment and protected-token diff
crates/chew-corpus/src/review.rs                # hash-bound doctor review decisions
crates/chew-corpus/src/report.rs                # JSON/Markdown reports and issue severity
crates/chew-corpus/src/pipeline.rs              # staged execution and atomic publication
crates/chew-corpus/tests/*.rs                    # public and CLI integration tests
tests/fixtures/corpus/*                          # small synthetic non-clinical fixtures
corpus/pipeline.json                             # explicit CHEW/JCHEW/CHO source pairs
corpus/manifests/*.json                         # six tracked source manifests
corpus/reviews/doses/.gitkeep                    # tracked review-record directory
corpus/derived/*                                 # ignored generated artifacts
```

---

### Task 1: Source manifests and explicit edition pairing

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/chew-corpus/Cargo.toml`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `crates/chew-corpus/src/config.rs`
- Create: `crates/chew-corpus/src/manifest.rs`
- Modify: `corpus/manifests/chew-2024.json`
- Modify: `corpus/manifests/jchew-2024.json`
- Modify: `corpus/manifests/cho-2024.json`
- Create: `corpus/manifests/chew-illustrated-2024.json`
- Create: `corpus/manifests/jchew-illustrated-2024.json`
- Create: `corpus/manifests/cho-illustrated-2024.json`
- Create: `corpus/pipeline.json`
- Test: `crates/chew-corpus/tests/config_manifest.rs`

**Interfaces:**
- Produces: `SourceCadre`, `Representation`, `SourceManifest`, `SourcePair`, and `PipelineConfig`.
- Produces: `PipelineConfig::load(repo_root, config_path) -> Result<PipelineConfig, CorpusError>`.
- Consumes: repository-relative manifest and PDF paths.

- [ ] **Step 1: Add the failing configuration and manifest tests**

Create `crates/chew-corpus/tests/config_manifest.rs`:

```rust
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
    ).unwrap();

    assert_eq!(manifest.source_id, "chew-2024-text");
    assert_eq!(manifest.cadre, SourceCadre::Chew);
    assert_eq!(manifest.representation, Representation::Text);
}

#[test]
fn repository_pipeline_has_three_explicit_pairs() {
    let config = PipelineConfig::load(Path::new("."), Path::new("corpus/pipeline.json"))
        .unwrap();

    assert_eq!(config.schema_version, 1);
    assert_eq!(config.pairs.len(), 3);
    assert!(config.pairs.iter().all(|pair| pair.text != pair.illustrated));
}
```

- [ ] **Step 2: Run the tests to verify the missing types fail**

Run: `cargo test -p chew-corpus --test config_manifest`

Expected: compilation fails because `SourceCadre`, `PipelineConfig`, `Representation`, and `SourceManifest` are not exported.

- [ ] **Step 3: Add dependencies and implement the typed source configuration**

Add these workspace dependencies to `Cargo.toml`:

```toml
clap = { version = "4", features = ["derive"] }
hex = "0.4"
quick-xml = "0.37"
regex = "1"
sha2 = "0.10"
tempfile = "3"
unicode-normalization = "0.1"
assert_cmd = "2"
```

Add the runtime dependencies with `.workspace = true` plus `serde.workspace = true`, `serde_json.workspace = true`, and `thiserror.workspace = true` to `[dependencies]` in `crates/chew-corpus/Cargo.toml`. Add `assert_cmd.workspace = true` under `[dev-dependencies]`.

Define in `manifest.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceCadre {
    #[serde(rename = "JCHEW")]
    Jchew,
    #[serde(rename = "CHEW")]
    Chew,
    #[serde(rename = "CHO")]
    Cho,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation { Text, Illustrated }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub source_id: String,
    pub title: String,
    pub cadre: SourceCadre,
    pub edition: String,
    pub representation: Representation,
    pub official_url: String,
    pub retrieved_at: String,
    pub byte_size: u64,
    pub sha256: String,
    pub page_count: u32,
    pub local_path: PathBuf,
}
```

Define in `config.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePair {
    pub cadre: SourceCadre,
    pub text: PathBuf,
    pub illustrated: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub schema_version: u32,
    pub derived_path: PathBuf,
    pub pairs: Vec<SourcePair>,
}
```

`PipelineConfig::load` must reject schema versions other than `1`, duplicate cadres, repeated manifest paths, pairs whose manifest cadres differ, or pairs whose representations are not text/illustrated.

- [ ] **Step 4: Add source IDs/representations and the illustrated manifests**

Use these illustrated values exactly:

| Source ID | Local path | URL suffix | Bytes | Pages | SHA-256 |
|---|---|---|---:|---:|---|
| `chew-2024-illustrated` | `corpus/raw/CHEW-Illustrated_compressed.pdf` | `CHEW-Illustrated_compressed.pdf` | 18795763 | 804 | `3ac0d17032ab4d1ff55077eacae711c4dc04e152db6dae7017a6a87d0cfce1f4` |
| `jchew-2024-illustrated` | `corpus/raw/JCHEW-Illustrated_compressed.pdf` | `JCHEW-Illustrated_compressed.pdf` | 18120794 | 804 | `d15cce13340a540c037e34ec072df4a8d682829129f9b9c6a3c3b89366ff8c06` |
| `cho-2024-illustrated` | `corpus/raw/CHO-Illustrated_compressed.pdf` | `CHO-Illustrated_compressed.pdf` | 21107943 | 906 | `6661c196f50a657c4e258787d837ac037a893fb2c0e85e5e7b6ded8c64d1f8cb` |

Use `retrieved_at: "2026-07-14"`, `edition: "2024"`, representation `illustrated`, and official URL prefix `https://chprbn.gov.ng/wp-content/uploads/2024/12/`.

Create `corpus/pipeline.json` with pairs ordered JCHEW, CHEW, CHO and `derived_path: "corpus/derived"`.

- [ ] **Step 5: Run tests and format checks**

Run: `cargo test -p chew-corpus --test config_manifest && cargo fmt --all -- --check`

Expected: both tests pass and formatter exits 0.

- [ ] **Step 6: Commit the metadata boundary**

```bash
git add Cargo.toml Cargo.lock crates/chew-corpus corpus/manifests corpus/pipeline.json
git commit -m "feat(corpus): define source manifests and edition pairs"
```

---

### Task 2: Source integrity and Poppler toolchain fingerprint

**Files:**
- Create: `crates/chew-corpus/src/error.rs`
- Extend: `crates/chew-corpus/src/manifest.rs`
- Create: `crates/chew-corpus/src/toolchain.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Test: `crates/chew-corpus/tests/integrity_toolchain.rs`
- Create: `tests/fixtures/corpus/source.bin`

**Interfaces:**
- Produces: `CorpusError`, `VerifiedSource`, `PopplerToolchain`, and `PdfMetadata`.
- Produces: `SourceManifest::verify(repo_root) -> Result<VerifiedSource, CorpusError>`.
- Produces: `PopplerToolchain::probe(pdfinfo, pdftotext) -> Result<Self, CorpusError>`.
- Produces: `PopplerToolchain::pdf_metadata(path) -> Result<PdfMetadata, CorpusError>`.

- [ ] **Step 1: Write failing integrity tests**

Create tests that write a manifest for `tests/fixtures/corpus/source.bin`, verify its known SHA-256, then change one expected hex digit and assert `CorpusError::ChecksumMismatch`. Add a fake `pdfinfo` executable fixture that prints:

```text
pdfinfo version 26.04.0
Pages: 2
Encrypted: no
```

Assert the toolchain returns version `26.04.0` and page count `2`.

- [ ] **Step 2: Run the tests to confirm the missing APIs fail**

Run: `cargo test -p chew-corpus --test integrity_toolchain`

Expected: compilation fails for missing `verify`, `PopplerToolchain`, and `CorpusError`.

- [ ] **Step 3: Implement streaming integrity verification**

Read files in 64 KiB chunks into `Sha256`, compare lowercase 64-character hex, byte size, and existence. Return:

```rust
pub struct VerifiedSource {
    pub manifest: SourceManifest,
    pub absolute_path: PathBuf,
    pub verified_sha256: String,
}
```

Reject non-HTTPS URLs, URLs outside `chprbn.gov.ng`, absolute `local_path` values, path traversal, invalid retrieval dates, zero pages, and encrypted PDFs.

- [ ] **Step 4: Implement process execution without a shell**

Use `std::process::Command` with argument arrays. Parse `pdfinfo -v` from stderr/stdout and `pdfinfo <path>` for `Pages`, `Encrypted`, `File size`, and `PDF version`. Include command, exit status, and bounded stderr in errors; never interpolate paths into a shell command.

- [ ] **Step 5: Run focused and crate tests**

Run: `cargo test -p chew-corpus --test integrity_toolchain && cargo test -p chew-corpus`

Expected: all tests pass.

- [ ] **Step 6: Commit integrity and toolchain validation**

```bash
git add crates/chew-corpus tests/fixtures/corpus/source.bin
git commit -m "feat(corpus): verify source integrity and Poppler tools"
```

---

### Task 3: Layout and bounding-box page extraction

**Files:**
- Create: `crates/chew-corpus/src/bbox.rs`
- Create: `crates/chew-corpus/src/page.rs`
- Extend: `crates/chew-corpus/src/toolchain.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `tests/fixtures/corpus/layout.txt`
- Create: `tests/fixtures/corpus/bbox.xhtml`
- Test: `crates/chew-corpus/tests/page_extraction.rs`

**Interfaces:**
- Produces: `BoundingPage`, `TextBlock`, `TextLine`, `Word`, `PageRecord`, and `ExtractionWarning`.
- Produces: `parse_bbox_layout(reader) -> Result<Vec<BoundingPage>, CorpusError>`.
- Produces: `build_page_records(source_id, layout, bounding_pages) -> Result<Vec<PageRecord>, CorpusError>`.
- Produces: `PopplerToolchain::extract(path, output_dir) -> Result<RawExtraction, CorpusError>`.

- [ ] **Step 1: Create two-page non-clinical fixtures and failing tests**

The layout fixture must use a form-feed between pages and visible headers `PAGE 7` and `PAGE 8`. The XHTML fixture must contain the same words with page/block/line/word bounding boxes.

Assert:

```rust
assert_eq!(pages.len(), 2);
assert_eq!(pages[0].physical_page, 1);
assert_eq!(pages[0].printed_page_label.as_deref(), Some("7"));
assert_eq!(pages[1].printed_page_label.as_deref(), Some("8"));
assert_eq!(pages[0].blocks[0].lines[0].words[0].text, "PAGE");
```

Also assert malformed XML, mismatched page counts, and non-finite/negative bounding boxes fail.

- [ ] **Step 2: Run the extraction test to verify red state**

Run: `cargo test -p chew-corpus --test page_extraction`

Expected: compilation fails because extraction types/functions do not exist.

- [ ] **Step 3: Parse Poppler XHTML with stable source order**

Use `quick_xml::Reader`, accept `page`, `flow`, `block`, `line`, and `word` elements, decode entities, and retain floating-point coordinates as finite `f32` values. Preserve document order; do not sort words geometrically after parsing.

Define `PageRecord` with schema version, source ID, physical page, optional printed label, width/height, exact layout text, bounding blocks, header/footer candidates, normalized content hash, and warnings.

- [ ] **Step 4: Implement fixed Poppler extraction commands**

Execute exactly:

```text
pdftotext -f 1 -l <pages> -layout -enc UTF-8 -eol unix <pdf> <layout-output>
pdftotext -f 1 -l <pages> -bbox-layout -enc UTF-8 -eol unix <pdf> <bbox-output>
```

Validate output existence, UTF-8, form-feed page count, XHTML page count, and source page count before returning.

- [ ] **Step 5: Run tests, clippy, and formatting**

Run: `cargo test -p chew-corpus --test page_extraction && cargo clippy -p chew-corpus --all-targets -- -D warnings && cargo fmt --all -- --check`

Expected: all commands exit 0.

- [ ] **Step 6: Commit page extraction**

```bash
git add crates/chew-corpus tests/fixtures/corpus/layout.txt tests/fixtures/corpus/bbox.xhtml
git commit -m "feat(corpus): extract layout-preserving page records"
```

---

### Task 4: Heading hierarchy and page-reference validation

**Files:**
- Create: `crates/chew-corpus/src/heading.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `tests/fixtures/corpus/headings-pages.json`
- Test: `crates/chew-corpus/tests/heading_validation.rs`

**Interfaces:**
- Produces: `HeadingRecord`, `HeadingLevel`, `TocReference`, `PageReferenceResult`, and `ValidationIssue`.
- Produces: `extract_headings(pages: &[PageRecord]) -> Vec<HeadingRecord>`.
- Produces: `extract_toc_references(pages: &[PageRecord]) -> Vec<TocReference>`.
- Produces: `validate_heading_references(headings, references) -> Vec<PageReferenceResult>`.

- [ ] **Step 1: Write failing hierarchy/reference tests**

Use synthetic headings `SECTION TWO`, `2.3 FEVER`, `2.3.1 SIMPLE FEVER` and TOC entries with printed pages. Assert normalized IDs remain `2.3` and `2.3.1`, repeated running headers are not headings, and a TOC entry pointing to the wrong printed page yields a blocking issue.

- [ ] **Step 2: Run tests to verify red state**

Run: `cargo test -p chew-corpus --test heading_validation`

Expected: compilation fails for missing heading APIs.

- [ ] **Step 3: Implement conservative heading detection**

Use anchored patterns for `SECTION <word-number>`, `1`, `1.2`, `1.2.3`, and numbered titles. Combine the pattern with line/block isolation and reject strings that end in dosage units, sentences, or running-header repetition. Preserve exact title text and store a Unicode/whitespace-normalized title separately.

- [ ] **Step 4: Implement TOC resolution and issue severity**

Resolve within one representation by section number first, normalized title second, and printed label last. Emit `missing_heading`, `duplicate_heading`, `page_reference_mismatch`, and `hierarchy_regression`; never silently choose among duplicate candidates.

- [ ] **Step 5: Run focused and crate verification**

Run: `cargo test -p chew-corpus --test heading_validation && cargo test -p chew-corpus`

Expected: all tests pass.

- [ ] **Step 6: Commit heading validation**

```bash
git add crates/chew-corpus tests/fixtures/corpus/headings-pages.json
git commit -m "feat(corpus): validate headings and printed page references"
```

---

### Task 5: Exhaustive dose-page inventory and candidate grouping

**Files:**
- Create: `crates/chew-corpus/src/dose.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `tests/fixtures/corpus/dose-pages.json`
- Test: `crates/chew-corpus/tests/dose_inventory.rs`

**Interfaces:**
- Produces: `DoseSignal`, `DoseLikePage`, `DoseCandidate`, `ComparisonStatus`, and `ClinicalReviewStatus`.
- Produces: `detect_dose_like_pages(pages: &[PageRecord]) -> Vec<DoseLikePage>`.
- Produces: `group_dose_candidates(pages, headings, dose_pages) -> Vec<DoseCandidate>`.
- Produces: `validate_dose_coverage(dose_pages, candidates, dismissals) -> Vec<ValidationIssue>`.

- [ ] **Step 1: Write failing detection and coverage tests**

Fixtures must cover `125 mg/5 mL`, `5 mg/kg IM stat`, `2.5 mL 8 hourly for 5 days`, `age 2–11 months`, `weight 5–14 kg`, a multi-page continuation, and false-positive prose containing only the word “dosage.”

Assert every signal retains exact source text and bounding-word indexes, multi-page continuations share one stable candidate ID, and an uncovered dose-like page yields a blocking `unaccounted_dose_page` issue.

- [ ] **Step 2: Run tests to verify red state**

Run: `cargo test -p chew-corpus --test dose_inventory`

Expected: compilation fails for missing dose APIs.

- [ ] **Step 3: Implement the high-recall dose-page detector**

Compile case-insensitive regexes once with `OnceLock`. Detect units, ratios, percentages, dose words, routes, timing, duration, age, weight, gestation, and action/treatment column headings. Store each matched exact substring, normalized category, line index, and word coordinates.

Never treat detector output as a normalized clinical dose.

- [ ] **Step 4: Implement deterministic candidate grouping**

Group adjacent dose-like pages under the closest preceding heading when continuation evidence exists. Derive candidate ID as `dose-<source_id>-<first_physical_page>-<first_16_hex_of_content_hash>`. Set:

```rust
comparison_status: ComparisonStatus::NotCompared,
clinical_review_status: ClinicalReviewStatus::PendingClinicalReview,
```

- [ ] **Step 5: Run tests and lint**

Run: `cargo test -p chew-corpus --test dose_inventory && cargo clippy -p chew-corpus --all-targets -- -D warnings`

Expected: all commands exit 0.

- [ ] **Step 6: Commit dose inventory**

```bash
git add crates/chew-corpus tests/fixtures/corpus/dose-pages.json
git commit -m "feat(corpus): inventory dose-like pages and candidates"
```

---

### Task 6: Edition alignment and protected-token comparison

**Files:**
- Create: `crates/chew-corpus/src/comparison.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `tests/fixtures/corpus/comparison-cases.json`
- Test: `crates/chew-corpus/tests/edition_comparison.rs`

**Interfaces:**
- Produces: `HeadingAlignment`, `CandidateComparison`, `ProtectedToken`, and `AlignmentStatus`.
- Produces: `align_headings(text, illustrated) -> Vec<HeadingAlignment>`.
- Produces: `compare_candidates(text, illustrated, alignments) -> Vec<CandidateComparison>`.
- Produces: `normalize_layout_text(input: &str) -> String` and `protected_tokens(input: &str) -> Vec<ProtectedToken>`.

- [ ] **Step 1: Write failing normalization and mismatch tests**

Assert whitespace, soft hyphens, line-break hyphenation, curly quotes, and Unicode dash variants can compare as format-only differences. Assert each of these changes is `content_mismatch`: `2.5 mL`→`5 mL`, `mg`→`mcg`, `IM`→`IV`, `8 hourly`→`12 hourly`, `5 days`→`3 days`, `5–14 kg`→`15–24 kg`, removed `not`, and removed `REFER`.

Add duplicate-heading input and assert `ambiguous_alignment` rather than first-match selection.

- [ ] **Step 2: Run tests to verify red state**

Run: `cargo test -p chew-corpus --test edition_comparison`

Expected: compilation fails for missing comparison APIs.

- [ ] **Step 3: Implement conservative normalization and protected tokens**

Use Unicode NFKC plus explicit whitespace/soft-hyphen handling. Tokenize numbers, fractions, ranges, units, drug-like words adjacent to strengths, routes, frequencies, durations, age/weight bands, negation, referral terms, and contraindication terms. Preserve original token strings and positions.

- [ ] **Step 4: Implement heading/candidate alignment**

Score section-number identity, normalized title identity, ordered neighboring headings, and rare protected-token anchors. Require a unique best candidate above a documented threshold; ties or one-to-many mappings are ambiguous. Do not score physical or printed page equality.

- [ ] **Step 5: Run focused tests and complete crate tests**

Run: `cargo test -p chew-corpus --test edition_comparison && cargo test -p chew-corpus`

Expected: all tests pass.

- [ ] **Step 6: Commit comparison behavior**

```bash
git add crates/chew-corpus tests/fixtures/corpus/comparison-cases.json
git commit -m "feat(corpus): compare text and illustrated dose content"
```

---

### Task 7: Hash-bound clinical review state

**Files:**
- Create: `crates/chew-corpus/src/review.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Create: `corpus/reviews/doses/.gitkeep`
- Test: `crates/chew-corpus/tests/clinical_review.rs`

**Interfaces:**
- Produces: `DoseReviewRecord`, `ReviewDecision`, and `EffectiveReviewStatus`.
- Produces: `apply_reviews(candidates, records) -> ReviewApplication`.
- Consumes: candidate ID, exact content hash, text/illustrated source IDs, reviewer, date, decision, and notes.

- [ ] **Step 1: Write failing review-state tests**

Assert no record means `PendingClinicalReview`; an `Approved` record with exact candidate/content/source hashes means `Approved`; any changed hash means `Stale` plus blocking issue; duplicate conflicting decisions mean `Conflict`; and engineering output cannot synthesize reviewer identity.

- [ ] **Step 2: Run tests to verify red state**

Run: `cargo test -p chew-corpus --test clinical_review`

Expected: compilation fails for missing review APIs.

- [ ] **Step 3: Implement immutable review records**

Define decisions `approved`, `rejected`, `needs_correction`, and `not_a_dosing_table`. Validate non-empty reviewer/notes, ISO `YYYY-MM-DD`, exact source IDs, exact 64-character hashes, and one effective decision per candidate hash.

Comparison status must never alter effective clinical review status.

- [ ] **Step 4: Run tests and serialization round trips**

Run: `cargo test -p chew-corpus --test clinical_review && cargo test -p chew-corpus`

Expected: all tests pass.

- [ ] **Step 5: Commit review contracts**

```bash
git add crates/chew-corpus corpus/reviews/doses/.gitkeep
git commit -m "feat(corpus): bind dose approvals to reviewed content hashes"
```

---

### Task 8: Deterministic artifact writer, reports, and CLI orchestration

**Files:**
- Create: `crates/chew-corpus/src/report.rs`
- Create: `crates/chew-corpus/src/pipeline.rs`
- Create: `crates/chew-corpus/src/main.rs`
- Modify: `crates/chew-corpus/src/lib.rs`
- Modify: `crates/chew-corpus/Cargo.toml`
- Test: `crates/chew-corpus/tests/report_pipeline.rs`
- Test: `crates/chew-corpus/tests/cli.rs`

**Interfaces:**
- Produces: `PipelineRunner::run(config_path) -> Result<RunReport, CorpusError>`.
- Produces: subcommands `extract`, `compare`, `run`, and `verify`.
- Produces the artifact tree and exit behavior defined in the design spec.

- [ ] **Step 1: Write failing deterministic-report tests**

Build an in-memory two-source result and assert `report.json` includes source/page/heading/dose/alignment/review counts, blocking issue details, and pending review queue. Assert Markdown contains source fingerprints, heading issues, dose coverage, mismatch table, and pending review headings.

Run twice with `SOURCE_DATE_EPOCH=1783987200` and assert identical semantic hashes and byte-identical JSONL/report outputs.

- [ ] **Step 2: Write failing CLI tests**

Using `assert_cmd`, assert `chew-corpus-pipeline --help` lists all four commands; missing config exits `2`; source mismatch and extraction failure exit non-zero; content mismatch still writes a report before returning the documented blocking exit.

- [ ] **Step 3: Run tests to verify red state**

Run: `cargo test -p chew-corpus --test report_pipeline --test cli`

Expected: compilation fails or binary is absent.

- [ ] **Step 4: Implement stable JSONL and Markdown writers**

Write one Serde struct per JSONL line with trailing newline, deterministic vector ordering, and no hash maps in serialized public artifacts. Hash artifact bytes with SHA-256. Include wall-clock `generated_at` only in `run.json`; use `SOURCE_DATE_EPOCH` when present and exclude the timestamp field from semantic digest input.

- [ ] **Step 5: Implement staged execution and atomic publication**

Create a temporary directory adjacent to `corpus/derived`, complete and verify the stage there, rename the prior generated directory to a backup, rename the completed stage into place, then remove the backup. On failure, retain the prior complete output and delete the incomplete stage.

Generated content must include `source.json`, `layout.txt`, `pages.jsonl`, `headings.jsonl`, `dose-candidates.jsonl`, per-cadre alignment/reference/comparison JSONL, `report.json`, `report.md`, and top-level `run.json`.

- [ ] **Step 6: Implement clap CLI and exit mapping**

Declare the binary in `crates/chew-corpus/Cargo.toml`:

```toml
[[bin]]
name = "chew-corpus-pipeline"
path = "src/main.rs"
```

Use default config `corpus/pipeline.json`, optional `--repo-root`, and no network options. Print a concise summary to stdout and issues to stderr without emitting raw full clinical pages to terminal logs.

- [ ] **Step 7: Run CLI, tests, lint, and formatting**

Run: `cargo test -p chew-corpus && cargo clippy -p chew-corpus --all-targets --all-features -- -D warnings && cargo fmt --all -- --check && cargo run -p chew-corpus --bin chew-corpus-pipeline -- --help`

Expected: tests pass, lint/format exit 0, and help lists `extract`, `compare`, `run`, `verify`.

- [ ] **Step 8: Commit orchestration and reporting**

```bash
git add Cargo.lock crates/chew-corpus
git commit -m "feat(corpus): orchestrate deterministic extraction reports"
```

---

### Task 9: Full six-source extraction and acceptance verification

**Files:**
- Generated/ignored: `corpus/derived/**`
- Modify if discrepancies require detector fixes: focused files under `crates/chew-corpus/src/`
- Create: `corpus/reviews/doses/README.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: all six verified PDFs and all prior task APIs.
- Produces: complete ignored artifacts plus contributor and doctor-review instructions.

- [ ] **Step 1: Run source verification only**

Run:

```bash
cargo run -p chew-corpus --bin chew-corpus-pipeline -- verify --config corpus/pipeline.json
```

Expected: six sources pass byte size, SHA-256, PDF page count, encryption, and toolchain checks; summary reports Poppler `26.04.0`.

- [ ] **Step 2: Run the full corpus pipeline with a fixed epoch**

Run:

```bash
SOURCE_DATE_EPOCH=1783987200 cargo run -p chew-corpus --bin chew-corpus-pipeline -- run --config corpus/pipeline.json
```

Expected: extraction completes for 372, 367, 410, 804, 804, and 906 pages; reports are generated even when clinical/content blocking issues exist.

- [ ] **Step 3: Inspect coverage invariants programmatically**

Run:

```bash
jq -e '.sources | length == 6' corpus/derived/run.json
jq -e '.dose_like_pages == (.dose_pages_accounted + .dose_pages_unaccounted)' corpus/derived/comparisons/CHEW/report.json
jq -e '.dose_like_pages == (.dose_pages_accounted + .dose_pages_unaccounted)' corpus/derived/comparisons/JCHEW/report.json
jq -e '.dose_like_pages == (.dose_pages_accounted + .dose_pages_unaccounted)' corpus/derived/comparisons/CHO/report.json
jq -e '.approved_dose_candidates == 0 and .pending_clinical_review == .dose_candidates' corpus/derived/comparisons/CHEW/report.json
jq -e '.approved_dose_candidates == 0 and .pending_clinical_review == .dose_candidates' corpus/derived/comparisons/JCHEW/report.json
jq -e '.approved_dose_candidates == 0 and .pending_clinical_review == .dose_candidates' corpus/derived/comparisons/CHO/report.json
```

Expected: every command prints `true` and exits 0. Any unaccounted page remains visibly blocking; it is not manually suppressed during engineering.

- [ ] **Step 4: Prove deterministic rerun output**

Copy the first run's semantic artifact digest from `corpus/derived/run.json`, rerun with the same `SOURCE_DATE_EPOCH`, and compare the new digest.

Run:

```bash
SOURCE_DATE_EPOCH=1783987200 cargo run -p chew-corpus --bin chew-corpus-pipeline -- run --config corpus/pipeline.json
jq -r '.semantic_artifact_sha256' corpus/derived/run.json
```

Expected: the digest exactly matches the first run.

- [ ] **Step 5: Document contributor and doctor workflows**

Add README commands for prerequisites, `verify`, `run`, ignored outputs, and reproducibility. Add `corpus/reviews/doses/README.md` documenting the JSON review schema, allowed decisions, content-hash invalidation, and that only the clinical lead may record `approved`.

- [ ] **Step 6: Run repository-wide verification**

Run:

```bash
pnpm check
cargo test --workspace
git diff --check
```

Expected: all commands exit 0. If `pnpm check` includes the full Cargo tests already, the explicit Cargo run still supplies fresh Rust evidence.

- [ ] **Step 7: Commit documentation and any evidence-driven fixes**

```bash
git add README.md corpus/reviews/doses/README.md crates/chew-corpus Cargo.lock
git commit -m "docs(corpus): document extraction and clinical review workflow"
```

Do not add `corpus/raw/` or `corpus/derived/`.

---

## Final Verification Checklist

- [ ] Six manifests verify against six official local PDFs.
- [ ] All 3,663 physical pages produce page records or explicit blocking issues.
- [ ] Text and illustrated printed page labels remain representation-specific.
- [ ] Every TOC reference resolves or is reported as blocking.
- [ ] Every dose-like page is accounted for or reported as blocking.
- [ ] Every dose candidate has an edition comparison record.
- [ ] Protected numeric/clinical token changes are content mismatches.
- [ ] Every dose candidate is pending until an exact-hash doctor review exists.
- [ ] CHO is never mapped to CHEW/JCHEW authorization.
- [ ] Identical fixed-epoch reruns have identical semantic artifact hashes.
- [ ] `pnpm check`, workspace tests, clippy, formatting, and `git diff --check` pass.
