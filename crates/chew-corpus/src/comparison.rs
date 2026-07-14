use std::{collections::HashSet, sync::OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::{ComparisonStatus, DoseCandidate, HeadingRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentStatus {
    Matched,
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingAlignment {
    pub text_section: String,
    pub illustrated_sections: Vec<String>,
    pub status: AlignmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedToken {
    pub normalized: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateComparison {
    pub text_candidate_id: String,
    pub illustrated_candidate_id: Option<String>,
    pub status: ComparisonStatus,
}

pub fn normalize_layout_text(input: &str) -> String {
    static HYPHEN: OnceLock<Regex> = OnceLock::new();
    let joined = HYPHEN
        .get_or_init(|| Regex::new(r"-\s*\n\s*").unwrap())
        .replace_all(input, "");
    joined
        .replace('\u{00ad}', "")
        .replace(['–', '—'], "-")
        .replace(['‘', '’'], "'")
        .replace(['“', '”'], "\"")
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

pub fn protected_tokens(input: &str) -> Vec<ProtectedToken> {
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let normalized = normalize_layout_text(input);
    let values: Vec<_> = TOKEN
        .get_or_init(|| Regex::new(r"[A-Z]+|\d+(?:\.\d+)?|[%/]").unwrap())
        .find_iter(&normalized)
        .map(|item| item.as_str())
        .collect();
    let protected: HashSet<&str> = [
        "MG",
        "MCG",
        "G",
        "ML",
        "IU",
        "KG",
        "IM",
        "IV",
        "SC",
        "ORAL",
        "TOPICAL",
        "INHALED",
        "RECTAL",
        "STAT",
        "HOURLY",
        "DAY",
        "DAYS",
        "MONTH",
        "MONTHS",
        "YEAR",
        "YEARS",
        "AGE",
        "WEIGHT",
        "NOT",
        "NO",
        "REFER",
        "CONTRAINDICATED",
    ]
    .into_iter()
    .collect();
    let mut indexes = HashSet::new();
    for (index, value) in values.iter().enumerate() {
        if value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
        {
            indexes.insert(index);
            if index > 0 && values[index - 1].chars().all(char::is_alphabetic) {
                indexes.insert(index - 1);
            }
        }
        if protected.contains(value) {
            indexes.insert(index);
        }
    }
    let mut indexes: Vec<_> = indexes.into_iter().collect();
    indexes.sort_unstable();
    indexes
        .into_iter()
        .map(|position| ProtectedToken {
            normalized: values[position].to_owned(),
            position,
        })
        .collect()
}

pub fn align_headings(
    text: &[HeadingRecord],
    illustrated: &[HeadingRecord],
) -> Vec<HeadingAlignment> {
    text.iter()
        .map(|heading| {
            let matches: Vec<_> = illustrated
                .iter()
                .filter(|candidate| {
                    candidate.section_number == heading.section_number
                        && candidate.normalized_title == heading.normalized_title
                })
                .collect();
            HeadingAlignment {
                text_section: heading.section_number.clone(),
                illustrated_sections: matches
                    .iter()
                    .map(|item| item.section_number.clone())
                    .collect(),
                status: match matches.len() {
                    0 => AlignmentStatus::Missing,
                    1 => AlignmentStatus::Matched,
                    _ => AlignmentStatus::Ambiguous,
                },
            }
        })
        .collect()
}

pub fn compare_candidates(
    text: &[DoseCandidate],
    illustrated: &[DoseCandidate],
    _alignments: &[HeadingAlignment],
) -> Vec<CandidateComparison> {
    text.iter()
        .map(|candidate| {
            let matches: Vec<_> = illustrated
                .iter()
                .filter(|other| other.heading_section == candidate.heading_section)
                .collect();
            let (id, status) = match matches.as_slice() {
                [] => (None, ComparisonStatus::MissingInIllustrated),
                [other] => {
                    let status = if protected_tokens(&candidate.exact_text)
                        .iter()
                        .map(|token| &token.normalized)
                        .collect::<Vec<_>>()
                        != protected_tokens(&other.exact_text)
                            .iter()
                            .map(|token| &token.normalized)
                            .collect::<Vec<_>>()
                    {
                        ComparisonStatus::ContentMismatch
                    } else if normalize_layout_text(&candidate.exact_text)
                        == normalize_layout_text(&other.exact_text)
                    {
                        ComparisonStatus::Match
                    } else {
                        ComparisonStatus::FormatOnlyDifference
                    };
                    (Some(other.candidate_id.clone()), status)
                }
                _ => (None, ComparisonStatus::AmbiguousAlignment),
            };
            CandidateComparison {
                text_candidate_id: candidate.candidate_id.clone(),
                illustrated_candidate_id: id,
                status,
            }
        })
        .collect()
}
