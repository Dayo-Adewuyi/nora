use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CorpusError, ValidationIssue};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSourceSummary {
    pub source_id: String,
    pub sha256: String,
    pub pages: u32,
    pub headings: usize,
    pub dose_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CadreReport {
    pub cadre: String,
    pub heading_alignments: usize,
    pub heading_issues: usize,
    pub dose_like_pages: usize,
    pub dose_pages_accounted: usize,
    pub dose_pages_unaccounted: usize,
    pub dose_candidates: usize,
    pub content_mismatches: usize,
    pub approved_dose_candidates: usize,
    pub pending_clinical_review: usize,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReport {
    pub schema_version: u32,
    pub generated_at: String,
    pub poppler_version: String,
    pub sources: Vec<RunSourceSummary>,
    pub comparisons: Vec<CadreReport>,
    pub blocking_issues: usize,
    pub semantic_artifact_sha256: String,
}

impl RunReport {
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "# Corpus extraction and comparison report\n\nPoppler: `{}`\n\n## Source fingerprints\n\n",
            self.poppler_version
        );
        for source in &self.sources {
            output.push_str(&format!(
                "- `{}` — `{}`; {} pages, {} headings, {} dose candidates\n",
                source.source_id,
                source.sha256,
                source.pages,
                source.headings,
                source.dose_candidates
            ));
        }
        output.push_str("\n## Heading validation\n\n");
        for cadre in &self.comparisons {
            output.push_str(&format!(
                "- {}: {} alignments; {} issues\n",
                cadre.cadre, cadre.heading_alignments, cadre.heading_issues
            ));
        }
        output.push_str("\n## Dose coverage\n\n");
        for cadre in &self.comparisons {
            output.push_str(&format!(
                "- {}: {} dose-like pages; {} accounted; {} unaccounted\n",
                cadre.cadre,
                cadre.dose_like_pages,
                cadre.dose_pages_accounted,
                cadre.dose_pages_unaccounted
            ));
        }
        output.push_str("\n## Content mismatches\n\n");
        for cadre in &self.comparisons {
            output.push_str(&format!(
                "- {}: {} protected-token mismatches\n",
                cadre.cadre, cadre.content_mismatches
            ));
        }
        output.push_str("\n## Pending clinical review\n\n");
        for cadre in &self.comparisons {
            output.push_str(&format!(
                "- {}: {} of {} candidates pending\n",
                cadre.cadre, cadre.pending_clinical_review, cadre.dose_candidates
            ));
        }
        output.push_str("\n## Blocking issues\n\n");
        for issue in self
            .comparisons
            .iter()
            .flat_map(|comparison| comparison.issues.iter())
            .filter(|issue| issue.blocking)
        {
            output.push_str(&format!("- `{}`: {}\n", issue.code, issue.message));
        }
        output
    }
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CorpusError> {
    ensure_parent(path)?;
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| CorpusError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|source| CorpusError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn write_jsonl<T: Serialize>(path: &Path, values: &[T]) -> Result<(), CorpusError> {
    ensure_parent(path)?;
    let mut bytes = Vec::new();
    for value in values {
        serde_json::to_writer(&mut bytes, value).map_err(|source| CorpusError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        bytes.push(b'\n');
    }
    fs::write(path, bytes).map_err(|source| CorpusError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn semantic_artifact_sha256(artifacts: &[(&str, &[u8])]) -> String {
    let mut artifacts = artifacts.to_vec();
    artifacts.sort_unstable_by_key(|(path, _)| *path);
    let mut digest = Sha256::new();
    for (path, bytes) in artifacts {
        digest.update((path.len() as u64).to_be_bytes());
        digest.update(path.as_bytes());
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    hex::encode(digest.finalize())
}

fn ensure_parent(path: &Path) -> Result<(), CorpusError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| CorpusError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}
