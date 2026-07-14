# CHEW Companion

Fully offline, refer-biased decision support for Nigerian CHEWs and JCHEWs, grounded in the 2024 National Standing Orders.

## Workspaces

- `apps/desktop`: Tauri 2 desktop host and React renderer.
- `crates/chew-domain`: shared cadre and clinical contracts.
- `crates/chew-corpus`: corpus extraction, comparison, and certification contracts.
- `crates/chew-inference`: local inference contracts.
- `crates/chew-benchmark`: performance budgets and benchmark tooling.
- `corpus`: official-source manifests, derived data, and clinical reviews.
- `models`: local GGUF and embedding models; model binaries are not committed.

## Rebuild the corpus reports

The pipeline is offline. Install Rust 1.88 or newer, `jq`, and Poppler 26.04.0 with `pdfinfo` and `pdftotext` on `PATH`. Place the six locally obtained PDFs at the paths declared in `corpus/manifests/*.json`; `corpus/raw/` and `corpus/derived/` are intentionally Git-ignored.

Verify all six immutable inputs before extraction:

```bash
cargo run -p chew-corpus --bin chew-corpus-pipeline -- verify --config corpus/pipeline.json
```

Generate layout text, bounding boxes, page and heading records, dosing candidates, cross-edition comparisons, and reports with a reproducible timestamp:

```bash
SOURCE_DATE_EPOCH=1783987200 cargo run -p chew-corpus --bin chew-corpus-pipeline -- run --config corpus/pipeline.json
```

The command publishes a complete `corpus/derived/` tree atomically. It exits non-zero when the report contains unresolved heading, page-reference, cross-edition, or clinical-review findings; that blocking exit is expected until the findings are resolved and the clinical lead signs off. Inspect `corpus/derived/run.json`, the top-level `report.md`, and each `corpus/derived/comparisons/<CADRE>/report.{json,md}`.

Two runs using the same source files, Poppler version, code, review records, and `SOURCE_DATE_EPOCH` must produce the same `semantic_artifact_sha256` in `corpus/derived/run.json`.

## Clinical safety boundary

Text editions remain the extraction and citation authority. Illustrated editions are comparison evidence for tables, dosing bands, and ambiguous layouts. Automated comparison never approves a dosing candidate. See `corpus/reviews/doses/README.md` for the hash-bound doctor-review workflow.
