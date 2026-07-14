use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{ClinicalReviewStatus, DoseCandidate, ValidationIssue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    Rejected,
    NeedsCorrection,
    NotADosingTable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoseReviewRecord {
    pub schema_version: u32,
    pub candidate_id: String,
    pub content_hash: String,
    pub text_source_id: String,
    pub illustrated_source_id: String,
    pub reviewer: String,
    pub reviewed_at: String,
    pub decision: ReviewDecision,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateReview {
    pub candidate_id: String,
    pub status: ClinicalReviewStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewApplication {
    pub candidates: Vec<CandidateReview>,
    pub issues: Vec<ValidationIssue>,
}

pub fn apply_reviews(
    candidates: &[DoseCandidate],
    records: &[DoseReviewRecord],
) -> ReviewApplication {
    let mut issues = Vec::new();
    let candidates = candidates
        .iter()
        .map(|candidate| {
            let for_candidate: Vec<_> = records
                .iter()
                .filter(|record| record.candidate_id == candidate.candidate_id)
                .collect();
            let exact: Vec<_> = for_candidate
                .iter()
                .copied()
                .filter(|record| record.content_hash == candidate.content_hash)
                .collect();
            let status = if exact.is_empty() {
                if for_candidate.is_empty() {
                    ClinicalReviewStatus::PendingClinicalReview
                } else {
                    issues.push(issue("stale_clinical_review", &candidate.candidate_id));
                    ClinicalReviewStatus::Stale
                }
            } else {
                let decisions: HashSet<_> = exact.iter().map(|record| record.decision).collect();
                if decisions.len() != 1 || exact.iter().any(|record| !valid_record(record)) {
                    issues.push(issue(
                        "conflicting_clinical_review",
                        &candidate.candidate_id,
                    ));
                    ClinicalReviewStatus::Conflict
                } else {
                    match exact[0].decision {
                        ReviewDecision::Approved => ClinicalReviewStatus::Approved,
                        ReviewDecision::Rejected => ClinicalReviewStatus::Rejected,
                        ReviewDecision::NeedsCorrection => ClinicalReviewStatus::NeedsCorrection,
                        ReviewDecision::NotADosingTable => ClinicalReviewStatus::NotADosingTable,
                    }
                }
            };
            CandidateReview {
                candidate_id: candidate.candidate_id.clone(),
                status,
            }
        })
        .collect();
    ReviewApplication { candidates, issues }
}

fn valid_record(record: &DoseReviewRecord) -> bool {
    record.schema_version == 1
        && !record.reviewer.trim().is_empty()
        && !record.notes.trim().is_empty()
        && record.reviewed_at.len() == 10
        && record.content_hash.len() == 64
        && !record.text_source_id.is_empty()
        && !record.illustrated_source_id.is_empty()
}
fn issue(code: &str, id: &str) -> ValidationIssue {
    ValidationIssue {
        code: code.into(),
        blocking: true,
        message: format!("clinical review for {id} is not valid for the current exact content"),
    }
}
