# Dose candidate clinical review

Every generated dose-table candidate starts as `pending_clinical_review`. Automated text/illustrated matches do not change that status. Only the designated clinical lead may record an `approved` decision.

Add one tracked JSON file per review under this directory using this schema:

```json
{
  "schema_version": 1,
  "candidate_id": "dose-chew-2024-text-100-0123456789abcdef",
  "content_hash": "64-lowercase-hex-characters-from-dose-candidates.jsonl",
  "text_source_id": "chew-2024-text",
  "illustrated_source_id": "chew-2024-illustrated",
  "reviewer": "Clinical lead name and credential",
  "reviewed_at": "2026-07-14",
  "decision": "approved",
  "notes": "Compared every drug, number, unit, route, frequency, duration, age/weight band, negation, referral instruction, and contraindication against both editions."
}
```

Allowed decisions are `approved`, `rejected`, `needs_correction`, and `not_a_dosing_table`. Reviewer, date, and notes are mandatory. The source IDs must identify the exact paired editions.

The `content_hash` binds a decision to the candidate's exact extracted text. Any extraction or source change that changes the hash makes the prior record `stale`; conflicting records are blocking. Never copy an approval to a new hash. Re-open both illustrated and text source pages, repeat the full clinical comparison, and issue a new review record.

Candidate IDs, content hashes, physical PDF pages, printed page labels, exact text, comparison status, and the pending queue are generated in:

```text
corpus/derived/sources/<text-source-id>/dose-candidates.jsonl
corpus/derived/comparisons/<CADRE>/dose-comparisons.jsonl
corpus/derived/comparisons/<CADRE>/report.json
```
