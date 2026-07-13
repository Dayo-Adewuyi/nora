# Corpus Extraction and Validation Pipeline Design

**Date:** 2026-07-14

**Status:** Approved for implementation

**Scope:** 2024 CHEW, JCHEW, and CHO text and illustrated Standing Orders

## 1. Objective

Build a deterministic, offline corpus pipeline that extracts layout-preserving text from all six official CHPRBN PDFs, records page and heading provenance, inventories every page containing dose-like content, aligns the text and illustrated representations, and produces a complete discrepancy report for clinical review.

The text editions remain the extraction and citation authority. The illustrated editions are comparison sources for headings, tables, dose bands, and ambiguous layouts. Automated agreement never grants clinical approval: every detected dosing table starts as `pending_clinical_review` and remains unusable for guided care until the doctor signs it off.

## 2. Scope boundaries

The pipeline will:

- Verify source manifests against the local PDFs before extraction.
- Preserve physical layout and word coordinates.
- Record physical PDF page numbers and printed page labels separately.
- Detect numbered headings and construct a heading hierarchy.
- Validate text-edition table-of-contents page references against extracted headings.
- Inventory dose-like pages and group dose-table candidates under source headings.
- Align equivalent content across text and illustrated editions by cadre, normalized heading, and content anchors.
- Compare clinically significant tokens without discarding the original strings.
- Produce JSONL artifacts, a machine-readable comparison report, and a Markdown review report.
- Require explicit clinical review records before any dose table is considered approved.

The pipeline will not:

- Infer a missing dose, unit, route, frequency, age band, or weight band.
- Treat layout similarity as clinical equivalence.
- Use CHO content to authorize CHEW or JCHEW actions.
- Promote protocols to `guided` status.
- OCR pages silently. A page without a usable text layer is a blocking extraction issue.
- Modify or overwrite files in `corpus/raw/`.

## 3. Architecture

### 3.1 Rust corpus CLI

The `chew-corpus` crate will expose a binary named `chew-corpus-pipeline`. The CLI owns validation, orchestration, parsing, normalization, alignment, comparison, report generation, and exit status.

Initial commands are:

```text
chew-corpus-pipeline extract --config corpus/pipeline.json
chew-corpus-pipeline compare --config corpus/pipeline.json
chew-corpus-pipeline run --config corpus/pipeline.json
chew-corpus-pipeline verify --config corpus/pipeline.json
```

`run` performs source verification, extraction, comparison, and artifact verification in order. Each stage writes to a temporary sibling directory and atomically replaces its generated destination only after the stage succeeds, so a failed run cannot leave apparently complete artifacts.

### 3.2 Poppler boundary

The CLI invokes the locally installed Poppler `pdftotext` executable with fixed options:

- `-layout` for a human-readable page representation.
- `-bbox-layout` for page, block, line, and word coordinates.
- UTF-8 output and Unix line endings.
- Explicit first and last pages derived from `pdfinfo`.

The run records the complete command options and Poppler version. An unsupported version, missing executable, non-zero exit, malformed bounding-box output, or page-count mismatch is a blocking error.

The implementation will be tested against Poppler 26.04.0. A different version may run, but it must be recorded and will produce a different toolchain fingerprint.

### 3.3 Source configuration and manifests

`corpus/pipeline.json` explicitly pairs each text edition with its illustrated edition. Pairing never depends on filename guessing.

Each source manifest contains:

- Stable `source_id`.
- Title and cadre.
- Edition and representation (`text` or `illustrated`).
- Official CHPRBN URL and retrieval date.
- Byte size, SHA-256 checksum, page count, and local path.

The existing text manifests will gain stable source identifiers and representation fields. Separate tracked manifests will be created for the three illustrated PDFs. Raw PDFs and generated extraction/comparison artifacts remain Git-ignored; manifests, pipeline configuration, clinical review records, schemas, and tests remain tracked.

## 4. Derived artifact layout

Generated files live under `corpus/derived/`:

```text
corpus/derived/
├── run.json
├── sources/
│   └── <source_id>/
│       ├── source.json
│       ├── layout.txt
│       ├── pages.jsonl
│       ├── headings.jsonl
│       └── dose-candidates.jsonl
└── comparisons/
    └── <cadre>/
        ├── heading-alignment.jsonl
        ├── page-reference-validation.jsonl
        ├── dose-comparison.jsonl
        ├── report.json
        └── report.md
```

Every JSON object has a schema version. JSON keys have stable ordering, arrays use deterministic source order, paths are repository-relative, and generated records contain no volatile identifiers.

`run.json` records input hashes, tool versions, command options, artifact hashes, and completion status. A wall-clock generation timestamp is metadata only and is excluded from the semantic artifact digest. `SOURCE_DATE_EPOCH` can fix that timestamp for byte-reproducibility checks.

## 5. Page and heading extraction

### 5.1 Page records

Each page record contains:

- `source_id`
- `physical_page`
- `printed_page_label`
- Page width and height
- Layout-preserved text
- Ordered blocks, lines, and words with bounding boxes
- Header and footer candidates
- Normalized content hash
- Extraction warnings

Physical pages are one-based PDF positions. Printed page labels are parsed from the visible header or footer and may differ between text and illustrated editions. The two values are never substituted for one another.

### 5.2 Heading records

Heading candidates are identified from numbered section patterns, typography/layout signals, repeated table-of-contents entries, and neighboring content. Each record preserves the exact source string and contains a conservative normalized form for alignment.

The heading hierarchy is validated for:

- Missing or duplicate numbered sections.
- Impossible section-number regressions.
- Headings referenced by the table of contents but absent from extracted pages.
- Printed page references that do not resolve to the referenced heading in the same representation.
- Material heading-title disagreement between text and illustrated editions.

The text-edition printed page remains the citation page. Illustrated physical and printed pages are stored only as comparison provenance.

## 6. Dosing-table inventory

### 6.1 Exhaustive dose-page coverage

The detector first creates a superset of dose-like pages using case-insensitive lexical and numeric signals, including:

- Dose and dosage terminology.
- Drug-strength patterns such as `mg`, `mcg`, `g`, `ml`, `mL`, `IU`, `%`, and ratios.
- Route terms such as oral, IM, IV, SC, topical, inhaled, and rectal.
- Frequency and duration patterns.
- Age, weight, gestation, and body-surface-area bands.
- Column headings commonly used in treatment and action tables.

Every dose-like page must be accounted for by one or more dose-table candidates or by a doctor-reviewed dismissal record. An unaccounted dose-like page is a blocking coverage issue. This superset approach favors false positives over missed dosing content.

### 6.2 Candidate records

A dose-table candidate may represent a visually bounded table, an action-column dose block, or a multi-page continuation. It contains:

- Stable candidate ID derived from source and content.
- Cadre, source heading, physical pages, and printed page labels.
- Exact layout-preserved source text.
- Word-coordinate references.
- Detected drug, strength, route, frequency, duration, age, and weight tokens.
- Continuation links for multi-page tables.
- Detector evidence and warnings.
- `comparison_status`.
- `clinical_review_status: pending_clinical_review`.

Detection does not convert prose into normalized clinical dose records. That normalization occurs only after clinical review.

## 7. Edition alignment and comparison

### 7.1 Alignment

Text and illustrated editions are aligned within the same cadre using:

1. Numbered heading identity.
2. Conservative heading normalization.
3. Ordered neighboring headings.
4. Rare content anchors such as drug names and distinctive clinical phrases.

Physical or printed page equality is not required because the representations have different layouts and page counts. Ambiguous, one-to-many, or missing alignments are reported and never silently selected.

### 7.2 Comparison normalization

Comparison may normalize:

- Unicode compatibility forms.
- Repeated whitespace and line wrapping.
- Soft hyphens and line-break hyphenation when the joined word is otherwise identical.
- Typographic quotation marks and dash code points while retaining their semantic placement.

Comparison must not normalize away:

- Numbers, decimals, fractions, ranges, or signs.
- Units or strengths.
- Drug names.
- Routes.
- Frequencies or durations.
- Age, weight, gestational, or population bands.
- Negation, referral language, or contraindications.

Each candidate receives one of these comparison statuses:

- `match`: exact clinically significant token sequence after permitted layout normalization.
- `format_only_difference`: significant tokens match but surrounding non-clinical wording or order differs.
- `content_mismatch`: any significant token differs.
- `missing_in_text` or `missing_in_illustrated`.
- `ambiguous_alignment`.
- `extraction_error`.

All statuses retain `clinical_review_status: pending_clinical_review` until the doctor signs an immutable review record.

## 8. Clinical review records

Tracked review records live under `corpus/reviews/doses/`. A record references the candidate ID, exact content hash, both source representations, reviewer identity, review date, decision, and notes.

Allowed decisions are:

- `approved`
- `rejected`
- `needs_correction`
- `not_a_dosing_table`

A review applies only to the exact candidate content hash. Re-extraction, source replacement, or candidate-content changes invalidate the prior approval and return the candidate to `pending_clinical_review`.

Engineering can generate and compare artifacts but cannot create an `approved` clinical decision on the doctor's behalf.

## 9. Reports and failure behavior

The machine-readable report includes counts and records for every source, page, heading, dose-like page, dose-table candidate, alignment, mismatch, coverage issue, and clinical-review state.

The Markdown report summarizes:

- Source and toolchain fingerprints.
- Extraction completeness by source.
- Heading and page-reference issues.
- Dose-like page coverage.
- Candidate comparison results.
- Blocking mismatches and ambiguous alignments.
- Pending clinical-review queue.
- Previously approved records invalidated by changed content.

The CLI exits non-zero for source-integrity failures, extraction failures, missing pages, unresolved table-of-contents references, unaccounted dose-like pages, malformed artifacts, or stale clinical approvals. Content mismatches are emitted as blocking review issues but do not prevent the report itself from being generated.

## 10. Testing strategy

Behavioral implementation follows test-driven development using small, non-clinical fixtures under `tests/fixtures/`.

Unit tests cover:

- Manifest and source-integrity validation.
- Bounding-box parsing and stable ordering.
- Printed-page-label parsing.
- Heading hierarchy and table-of-contents resolution.
- Dose-like token detection.
- Multi-page candidate grouping.
- Permitted and forbidden comparison normalization.
- Alignment ambiguity.
- Review invalidation when a content hash changes.
- Stable IDs and semantic digests.

Integration tests use synthetic PDFs or checked-in tiny fixtures to verify Poppler orchestration, atomic output replacement, deterministic reruns, and failure cleanup.

Corpus acceptance checks run against all six official PDFs and require:

- Source sizes and hashes match their manifests.
- Extracted page counts match `pdfinfo`.
- Every page produces a page record or a blocking issue.
- Every table-of-contents heading resolves or appears in the report as blocking.
- Every dose-like page is covered by a candidate or reviewed dismissal.
- Every candidate appears in the comparison report.
- Every candidate remains `pending_clinical_review` without a valid doctor review record.
- Two runs with the same inputs, Poppler version, options, and `SOURCE_DATE_EPOCH` produce identical semantic artifact hashes.

## 11. Completion criteria

This work is complete when:

1. The CLI reproducibly extracts all six sources into the documented artifact layout.
2. Source, page, heading, and table-of-contents validation runs without silent omissions.
3. The comparison report inventories all dose-like pages and every detected dosing table across CHEW, JCHEW, and CHO.
4. Numeric, unit, drug, route, frequency, duration, age, and weight differences are visible as blocking issues.
5. Every dosing table is `pending_clinical_review` unless an exact-hash doctor review record approves or dismisses it.
6. The report gives the doctor enough exact text, coordinates, page references, and source links to adjudicate each item against both editions.
7. Automated tests and full-corpus verification pass with recorded evidence.
