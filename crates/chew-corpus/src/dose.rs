use std::{collections::HashSet, sync::OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{HeadingRecord, PageRecord, ValidationIssue};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoseSignal {
    pub category: String,
    pub exact_text: String,
    pub line_index: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoseLikePage {
    pub source_id: String,
    pub physical_page: u32,
    pub signals: Vec<DoseSignal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    NotCompared,
    Match,
    FormatOnlyDifference,
    ContentMismatch,
    MissingInText,
    MissingInIllustrated,
    AmbiguousAlignment,
    ExtractionError,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClinicalReviewStatus {
    PendingClinicalReview,
    Approved,
    Rejected,
    NeedsCorrection,
    NotADosingTable,
    Stale,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoseCandidate {
    pub schema_version: u32,
    pub candidate_id: String,
    pub source_id: String,
    pub heading_section: Option<String>,
    pub physical_pages: Vec<u32>,
    pub printed_page_labels: Vec<String>,
    pub exact_text: String,
    pub content_hash: String,
    pub signals: Vec<DoseSignal>,
    pub comparison_status: ComparisonStatus,
    pub clinical_review_status: ClinicalReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DosePageDismissal {
    pub physical_page: u32,
}

pub fn detect_dose_like_pages(pages: &[PageRecord]) -> Vec<DoseLikePage> {
    pages
        .iter()
        .filter_map(|page| {
            let mut signals = Vec::new();
            for (line_index, line) in page.layout_text.lines().enumerate() {
                collect(
                    "strength",
                    strength_pattern(),
                    line,
                    line_index,
                    &mut signals,
                );
                collect("timing", timing_pattern(), line, line_index, &mut signals);
                collect("band", band_pattern(), line, line_index, &mut signals);
                collect("route", route_pattern(), line, line_index, &mut signals);
            }
            let strong = signals.iter().any(|signal| signal.category == "strength");
            (strong || signals.len() >= 2).then(|| DoseLikePage {
                source_id: page.source_id.clone(),
                physical_page: page.physical_page,
                signals,
            })
        })
        .collect()
}

pub fn group_dose_candidates(
    pages: &[PageRecord],
    headings: &[HeadingRecord],
    dose_pages: &[DoseLikePage],
) -> Vec<DoseCandidate> {
    let mut groups: Vec<Vec<&DoseLikePage>> = Vec::new();
    for dose_page in dose_pages {
        if groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|last| {
                last.source_id == dose_page.source_id
                    && last.physical_page + 1 == dose_page.physical_page
            })
        {
            groups.last_mut().unwrap().push(dose_page)
        } else {
            groups.push(vec![dose_page]);
        }
    }
    groups
        .into_iter()
        .map(|group| {
            let first = group[0];
            let group_pages: HashSet<u32> = group.iter().map(|page| page.physical_page).collect();
            let selected: Vec<_> = pages
                .iter()
                .filter(|page| {
                    page.source_id == first.source_id && group_pages.contains(&page.physical_page)
                })
                .collect();
            let exact_text = selected
                .iter()
                .map(|page| page.layout_text.as_str())
                .collect::<Vec<_>>()
                .join("\n\x0c\n");
            let content_hash = hex::encode(Sha256::digest(exact_text.as_bytes()));
            let heading = headings
                .iter()
                .filter(|heading| {
                    heading.source_id == first.source_id
                        && heading.physical_page <= first.physical_page
                })
                .max_by_key(|heading| heading.physical_page);
            DoseCandidate {
                schema_version: 1,
                candidate_id: format!(
                    "dose-{}-{}-{}",
                    first.source_id,
                    first.physical_page,
                    &content_hash[..16]
                ),
                source_id: first.source_id.clone(),
                heading_section: heading.map(|heading| heading.section_number.clone()),
                physical_pages: selected.iter().map(|page| page.physical_page).collect(),
                printed_page_labels: selected
                    .iter()
                    .filter_map(|page| page.printed_page_label.clone())
                    .collect(),
                exact_text,
                content_hash,
                signals: group
                    .into_iter()
                    .flat_map(|page| page.signals.clone())
                    .collect(),
                comparison_status: ComparisonStatus::NotCompared,
                clinical_review_status: ClinicalReviewStatus::PendingClinicalReview,
            }
        })
        .collect()
}

pub fn validate_dose_coverage(
    dose_pages: &[DoseLikePage],
    candidates: &[DoseCandidate],
    dismissals: &[DosePageDismissal],
) -> Vec<ValidationIssue> {
    let accounted: HashSet<u32> = candidates
        .iter()
        .flat_map(|candidate| candidate.physical_pages.iter().copied())
        .chain(dismissals.iter().map(|dismissal| dismissal.physical_page))
        .collect();
    dose_pages
        .iter()
        .filter(|page| !accounted.contains(&page.physical_page))
        .map(|page| ValidationIssue {
            code: "unaccounted_dose_page".into(),
            blocking: true,
            message: format!(
                "dose-like page {} is not assigned to a candidate or dismissal",
                page.physical_page
            ),
        })
        .collect()
}

fn collect(
    category: &str,
    pattern: &Regex,
    line: &str,
    line_index: usize,
    signals: &mut Vec<DoseSignal>,
) {
    signals.extend(pattern.find_iter(line).map(|found| DoseSignal {
        category: category.into(),
        exact_text: found.as_str().into(),
        line_index,
    }));
}
fn strength_pattern() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(||Regex::new(r"(?i)\b\d+(?:\.\d+)?\s*(?:mcg|mg|g|ml|iu|%)(?:\s*/\s*(?:\d+(?:\.\d+)?\s*)?(?:mcg|mg|g|ml|kg))?").unwrap())
}
fn timing_pattern() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\b(?:stat|\d+\s*hourly|for\s+\d+\s+days?)\b").unwrap())
}
fn band_pattern() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(||Regex::new(r"(?i)\b(?:age|weight)?\s*\d+(?:\.\d+)?\s*[-–]\s*\d+(?:\.\d+)?\s*(?:months?|years?|kg)\b").unwrap())
}
fn route_pattern() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\b(?:oral|IM|IV|SC|topical|inhaled|rectal)\b").unwrap())
}
