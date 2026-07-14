use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    align_headings, apply_reviews, build_page_records, compare_candidates, detect_dose_like_pages,
    extract_headings, extract_toc_references, group_dose_candidates, semantic_artifact_sha256,
    validate_dose_coverage, validate_heading_hierarchy, validate_heading_references, write_json,
    write_jsonl, CadreReport, CandidateComparison, ClinicalReviewStatus, ComparisonStatus,
    CorpusError, DoseCandidate, DoseReviewRecord, HeadingRecord, PageRecord, PageReferenceResult,
    PipelineConfig, PopplerToolchain, RunReport, RunSourceSummary, SourceCadre, SourceManifest,
    ValidationIssue, VerifiedSource,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    pub schema_version: u32,
    pub poppler_version: String,
    pub sources: Vec<RunSourceSummary>,
}

pub struct PipelineRunner {
    repo_root: PathBuf,
    toolchain: PopplerToolchain,
}

struct VerifiedPair {
    cadre: SourceCadre,
    text: VerifiedSource,
    illustrated: VerifiedSource,
}

struct ExtractedSource {
    manifest: SourceManifest,
    pages: Vec<PageRecord>,
    headings: Vec<HeadingRecord>,
    candidates: Vec<DoseCandidate>,
}

impl PipelineRunner {
    pub fn new(repo_root: PathBuf) -> Result<Self, CorpusError> {
        let toolchain =
            PopplerToolchain::probe(PathBuf::from("pdfinfo"), PathBuf::from("pdftotext"))?;
        Ok(Self::from_toolchain(repo_root, toolchain))
    }

    pub fn from_toolchain(repo_root: PathBuf, toolchain: PopplerToolchain) -> Self {
        Self {
            repo_root,
            toolchain,
        }
    }

    pub fn verify(&self, config_path: &Path) -> Result<VerificationReport, CorpusError> {
        let config = PipelineConfig::load(&self.repo_root, config_path)?;
        let pairs = self.verify_pairs(&config)?;
        Ok(VerificationReport {
            schema_version: 1,
            poppler_version: self.toolchain.version.clone(),
            sources: source_summaries(&pairs),
        })
    }

    pub fn run(&self, config_path: &Path) -> Result<RunReport, CorpusError> {
        let config = PipelineConfig::load(&self.repo_root, config_path)?;
        let pairs = self.verify_pairs(&config)?;
        let destination = self.repo_root.join(&config.derived_path);
        let stage = destination.with_extension(format!("stage-{}", std::process::id()));
        if stage.exists() {
            fs::remove_dir_all(&stage).map_err(|source| CorpusError::Write {
                path: stage.clone(),
                source,
            })?;
        }
        fs::create_dir_all(&stage).map_err(|source| CorpusError::Write {
            path: stage.clone(),
            source,
        })?;
        let result = self.build_stage(&stage, &pairs);
        match result {
            Ok(report) => {
                publish(&stage, &destination)?;
                Ok(report)
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&stage);
                Err(error)
            }
        }
    }

    fn verify_pairs(&self, config: &PipelineConfig) -> Result<Vec<VerifiedPair>, CorpusError> {
        config
            .manifests(&self.repo_root)?
            .into_iter()
            .map(|(cadre, text, illustrated)| {
                Ok(VerifiedPair {
                    cadre,
                    text: self.verify_source(text)?,
                    illustrated: self.verify_source(illustrated)?,
                })
            })
            .collect()
    }

    fn verify_source(&self, manifest: SourceManifest) -> Result<VerifiedSource, CorpusError> {
        let verified = manifest.verify(&self.repo_root)?;
        let metadata = self.toolchain.pdf_metadata(&verified.absolute_path)?;
        if metadata.encrypted {
            return Err(CorpusError::InvalidManifest(format!(
                "{} is encrypted",
                manifest.source_id
            )));
        }
        if metadata.pages != manifest.page_count || metadata.file_size != manifest.byte_size {
            return Err(CorpusError::InvalidManifest(format!(
                "{} PDF metadata differs from its manifest",
                manifest.source_id
            )));
        }
        Ok(verified)
    }

    fn build_stage(&self, stage: &Path, pairs: &[VerifiedPair]) -> Result<RunReport, CorpusError> {
        fs::write(stage.join(".gitkeep"), []).map_err(|source| CorpusError::Write {
            path: stage.join(".gitkeep"),
            source,
        })?;
        let reviews = load_review_records(&self.repo_root.join("corpus/reviews/doses"))?;
        let mut sources = Vec::new();
        let mut comparisons = Vec::new();
        for pair in pairs {
            let mut text = self.extract_source(stage, &pair.text)?;
            let illustrated = self.extract_source(stage, &pair.illustrated)?;
            let cadre = pair.cadre.as_str();
            let comparison_dir = stage.join("comparisons").join(cadre);

            let alignments = align_headings(&text.headings, &illustrated.headings);
            let candidate_comparisons =
                compare_candidates(&text.candidates, &illustrated.candidates, &alignments);
            apply_comparison_statuses(&mut text.candidates, &candidate_comparisons);
            let reviews_applied = apply_reviews(&text.candidates, &reviews);
            for candidate in &mut text.candidates {
                if let Some(review) = reviews_applied
                    .candidates
                    .iter()
                    .find(|review| review.candidate_id == candidate.candidate_id)
                {
                    candidate.clinical_review_status = review.status;
                }
            }

            let text_dose_pages = detect_dose_like_pages(&text.pages);
            let mut issues = source_heading_issues(&text);
            issues.extend(source_heading_issues(&illustrated));
            issues.extend(validate_dose_coverage(
                &text_dose_pages,
                &text.candidates,
                &[],
            ));
            issues.extend(reviews_applied.issues);
            issues.extend(comparison_issues(&candidate_comparisons));
            let reference_results = source_reference_results(&text)
                .into_iter()
                .chain(source_reference_results(&illustrated))
                .collect::<Vec<_>>();
            issues.extend(
                reference_results
                    .iter()
                    .filter_map(|result| result.issue.clone()),
            );

            let accounted = text
                .candidates
                .iter()
                .flat_map(|candidate| candidate.physical_pages.iter())
                .collect::<std::collections::HashSet<_>>()
                .len();
            let report = CadreReport {
                cadre: cadre.into(),
                heading_alignments: alignments.len(),
                heading_issues: issues
                    .iter()
                    .filter(|issue| {
                        issue.code.contains("heading")
                            || issue.code.contains("hierarchy")
                            || issue.code.contains("reference")
                    })
                    .count(),
                dose_like_pages: text_dose_pages.len(),
                dose_pages_accounted: accounted,
                dose_pages_unaccounted: text_dose_pages.len().saturating_sub(accounted),
                dose_candidates: text.candidates.len(),
                content_mismatches: candidate_comparisons
                    .iter()
                    .filter(|item| item.status == ComparisonStatus::ContentMismatch)
                    .count(),
                approved_dose_candidates: text
                    .candidates
                    .iter()
                    .filter(|item| item.clinical_review_status == ClinicalReviewStatus::Approved)
                    .count(),
                pending_clinical_review: text
                    .candidates
                    .iter()
                    .filter(|item| {
                        item.clinical_review_status == ClinicalReviewStatus::PendingClinicalReview
                    })
                    .count(),
                issues,
            };
            write_jsonl(
                &comparison_dir.join("heading-alignments.jsonl"),
                &alignments,
            )?;
            write_jsonl(
                &comparison_dir.join("reference-validation.jsonl"),
                &reference_results,
            )?;
            write_jsonl(
                &comparison_dir.join("dose-comparisons.jsonl"),
                &candidate_comparisons,
            )?;
            write_json(&comparison_dir.join("report.json"), &report)?;
            fs::write(comparison_dir.join("report.md"), cadre_markdown(&report)).map_err(
                |source| CorpusError::Write {
                    path: comparison_dir.join("report.md"),
                    source,
                },
            )?;
            // Re-publish text candidates after comparison and review statuses are applied.
            write_jsonl(
                &stage
                    .join("sources")
                    .join(&text.manifest.source_id)
                    .join("dose-candidates.jsonl"),
                &text.candidates,
            )?;
            comparisons.push(report);
            sources.push(summary(&text));
            sources.push(summary(&illustrated));
        }

        let blocking_issues = comparisons
            .iter()
            .flat_map(|comparison| &comparison.issues)
            .filter(|issue| issue.blocking)
            .count();
        let mut report = RunReport {
            schema_version: 1,
            generated_at: generated_at(),
            poppler_version: self.toolchain.version.clone(),
            sources,
            comparisons,
            blocking_issues,
            semantic_artifact_sha256: String::new(),
        };
        fs::write(stage.join("report.md"), report.to_markdown()).map_err(|source| {
            CorpusError::Write {
                path: stage.join("report.md"),
                source,
            }
        })?;
        let artifacts = artifact_bytes(stage)?;
        let borrowed = artifacts
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>();
        report.semantic_artifact_sha256 = semantic_artifact_sha256(&borrowed);
        write_json(&stage.join("run.json"), &report)?;
        Ok(report)
    }

    fn extract_source(
        &self,
        stage: &Path,
        source: &VerifiedSource,
    ) -> Result<ExtractedSource, CorpusError> {
        let output = stage.join("sources").join(&source.manifest.source_id);
        fs::create_dir_all(&output).map_err(|io| CorpusError::Write {
            path: output.clone(),
            source: io,
        })?;
        write_json(&output.join("source.json"), &source.manifest)?;
        let raw = self.toolchain.extract(&source.absolute_path, &output)?;
        let layout = raw.layout_pages.join("\x0c");
        let pages = build_page_records(&source.manifest.source_id, &layout, raw.bounding_pages)?;
        let headings = extract_headings(&pages);
        let dose_pages = detect_dose_like_pages(&pages);
        let candidates = group_dose_candidates(&pages, &headings, &dose_pages);
        write_jsonl(&output.join("pages.jsonl"), &pages)?;
        write_jsonl(&output.join("headings.jsonl"), &headings)?;
        write_jsonl(&output.join("dose-like-pages.jsonl"), &dose_pages)?;
        write_jsonl(&output.join("dose-candidates.jsonl"), &candidates)?;
        Ok(ExtractedSource {
            manifest: source.manifest.clone(),
            pages,
            headings,
            candidates,
        })
    }
}

fn source_summaries(pairs: &[VerifiedPair]) -> Vec<RunSourceSummary> {
    pairs
        .iter()
        .flat_map(|pair| [&pair.text, &pair.illustrated])
        .map(|source| RunSourceSummary {
            source_id: source.manifest.source_id.clone(),
            sha256: source.verified_sha256.clone(),
            pages: source.manifest.page_count,
            headings: 0,
            dose_candidates: 0,
        })
        .collect()
}

fn summary(source: &ExtractedSource) -> RunSourceSummary {
    RunSourceSummary {
        source_id: source.manifest.source_id.clone(),
        sha256: source.manifest.sha256.clone(),
        pages: source.pages.len() as u32,
        headings: source.headings.len(),
        dose_candidates: source.candidates.len(),
    }
}

fn source_heading_issues(source: &ExtractedSource) -> Vec<ValidationIssue> {
    validate_heading_hierarchy(&source.headings)
}

fn source_reference_results(source: &ExtractedSource) -> Vec<PageReferenceResult> {
    validate_heading_references(&source.headings, &extract_toc_references(&source.pages))
}

fn apply_comparison_statuses(
    candidates: &mut [DoseCandidate],
    comparisons: &[CandidateComparison],
) {
    for candidate in candidates {
        if let Some(comparison) = comparisons
            .iter()
            .find(|item| item.text_candidate_id == candidate.candidate_id)
        {
            candidate.comparison_status = comparison.status;
        }
    }
}

fn comparison_issues(comparisons: &[CandidateComparison]) -> Vec<ValidationIssue> {
    comparisons
        .iter()
        .filter(|comparison| {
            !matches!(
                comparison.status,
                ComparisonStatus::Match | ComparisonStatus::FormatOnlyDifference
            )
        })
        .map(|comparison| ValidationIssue {
            code: format!("dose_{:?}", comparison.status).to_lowercase(),
            blocking: true,
            message: format!(
                "{} has comparison status {:?}",
                comparison.text_candidate_id, comparison.status
            ),
        })
        .collect()
}

fn load_review_records(path: &Path) -> Result<Vec<DoseReviewRecord>, CorpusError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(path)
        .map_err(|source| CorpusError::Read {
            path: path.to_path_buf(),
            source,
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let bytes = fs::read(&file).map_err(|source| CorpusError::Read {
                path: file.clone(),
                source,
            })?;
            serde_json::from_slice(&bytes)
                .map_err(|source| CorpusError::Json { path: file, source })
        })
        .collect()
}

fn cadre_markdown(report: &CadreReport) -> String {
    format!(
        "# {} comparison\n\n- Dose-like pages: {}\n- Accounted: {}\n- Unaccounted: {}\n- Dose candidates: {}\n- Pending clinical review: {}\n- Content mismatches: {}\n",
        report.cadre,
        report.dose_like_pages,
        report.dose_pages_accounted,
        report.dose_pages_unaccounted,
        report.dose_candidates,
        report.pending_clinical_review,
        report.content_mismatches
    )
}

fn generated_at() -> String {
    let seconds = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
    format!("unix:{seconds}")
}

fn artifact_bytes(root: &Path) -> Result<Vec<(String, Vec<u8>)>, CorpusError> {
    fn visit(
        root: &Path,
        path: &Path,
        output: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), CorpusError> {
        let mut entries = fs::read_dir(path)
            .map_err(|source| CorpusError::Read {
                path: path.to_path_buf(),
                source,
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| CorpusError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let child = entry.path();
            if child.is_dir() {
                visit(root, &child, output)?;
            } else if child
                .file_name()
                .is_some_and(|name| name != "run.json" && name != ".gitkeep")
            {
                let relative = child
                    .strip_prefix(root)
                    .expect("artifact is under stage")
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = fs::read(&child).map_err(|source| CorpusError::Read {
                    path: child,
                    source,
                })?;
                output.push((relative, bytes));
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn publish(stage: &Path, destination: &Path) -> Result<(), CorpusError> {
    let backup = destination.with_extension(format!("backup-{}", std::process::id()));
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|source| CorpusError::Write {
            path: backup.clone(),
            source,
        })?;
    }
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|source| CorpusError::Write {
            path: destination.to_path_buf(),
            source,
        })?;
    }
    if let Err(source) = fs::rename(stage, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(CorpusError::Write {
            path: destination.to_path_buf(),
            source,
        });
    }
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|source| CorpusError::Write {
            path: backup,
            source,
        })?;
    }
    Ok(())
}
