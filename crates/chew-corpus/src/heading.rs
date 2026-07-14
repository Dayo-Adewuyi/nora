use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::PageRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadingLevel {
    Section,
    Numbered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingRecord {
    pub schema_version: u32,
    pub source_id: String,
    pub section_number: String,
    pub exact_title: String,
    pub normalized_title: String,
    pub level: HeadingLevel,
    pub physical_page: u32,
    pub printed_page_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocReference {
    pub section_number: String,
    pub title: String,
    pub printed_page_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub blocking: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageReferenceResult {
    pub section_number: String,
    pub issue: Option<ValidationIssue>,
}

pub fn extract_headings(pages: &[PageRecord]) -> Vec<HeadingRecord> {
    let pattern = heading_pattern();
    pages
        .iter()
        .filter(|page| {
            !page
                .layout_text
                .to_uppercase()
                .contains("TABLE OF CONTENTS")
        })
        .flat_map(|page| {
            page.layout_text.lines().filter_map(move |line| {
                let trimmed = line.trim();
                if section_pattern().is_match(trimmed) {
                    return Some(HeadingRecord {
                        schema_version: 1,
                        source_id: page.source_id.clone(),
                        section_number: trimmed.to_uppercase(),
                        exact_title: trimmed.to_owned(),
                        normalized_title: normalize(trimmed),
                        level: HeadingLevel::Section,
                        physical_page: page.physical_page,
                        printed_page_label: page.printed_page_label.clone(),
                    });
                }
                let captures = pattern.captures(line)?;
                let number = captures.get(1)?.as_str().trim_end_matches('.');
                let title = captures.get(2)?.as_str().trim();
                if !number.contains('.') && title != title.to_uppercase() {
                    return None;
                }
                Some(HeadingRecord {
                    schema_version: 1,
                    source_id: page.source_id.clone(),
                    section_number: number.to_owned(),
                    exact_title: title.to_owned(),
                    normalized_title: normalize(title),
                    level: HeadingLevel::Numbered,
                    physical_page: page.physical_page,
                    printed_page_label: page.printed_page_label.clone(),
                })
            })
        })
        .collect()
}

pub fn validate_heading_hierarchy(headings: &[HeadingRecord]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut previous: Option<Vec<u32>> = None;
    for heading in headings
        .iter()
        .filter(|heading| heading.level == HeadingLevel::Numbered)
    {
        let current: Vec<u32> = heading
            .section_number
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect();
        if previous.as_ref().is_some_and(|prior| current < *prior) {
            issues.push(issue(
                "hierarchy_regression",
                format!(
                    "{} appears after a later numbered heading",
                    heading.section_number
                ),
            ));
        }
        previous = Some(current);
    }
    issues
}

pub fn extract_toc_references(pages: &[PageRecord]) -> Vec<TocReference> {
    let pattern = toc_pattern();
    pages
        .iter()
        .filter(|page| {
            page.layout_text
                .to_uppercase()
                .contains("TABLE OF CONTENTS")
        })
        .flat_map(|page| {
            page.layout_text.lines().filter_map(|line| {
                let captures = pattern.captures(line)?;
                Some(TocReference {
                    section_number: captures.get(1)?.as_str().trim_end_matches('.').to_owned(),
                    title: captures.get(2)?.as_str().trim().to_owned(),
                    printed_page_label: captures.get(3)?.as_str().to_owned(),
                })
            })
        })
        .collect()
}

pub fn validate_heading_references(
    headings: &[HeadingRecord],
    references: &[TocReference],
) -> Vec<PageReferenceResult> {
    references
        .iter()
        .map(|reference| {
            let matches: Vec<_> = headings
                .iter()
                .filter(|heading| heading.section_number == reference.section_number)
                .collect();
            let issue = match matches.as_slice() {
                [] => Some(issue(
                    "missing_heading",
                    format!(
                        "{} is referenced by the TOC but missing",
                        reference.section_number
                    ),
                )),
                [heading]
                    if heading.printed_page_label.as_deref()
                        != Some(reference.printed_page_label.as_str()) =>
                {
                    Some(issue(
                        "page_reference_mismatch",
                        format!(
                            "{} points to {}, heading is on {:?}",
                            reference.section_number,
                            reference.printed_page_label,
                            heading.printed_page_label
                        ),
                    ))
                }
                [_] => None,
                _ => Some(issue(
                    "duplicate_heading",
                    format!("{} resolves to multiple headings", reference.section_number),
                )),
            };
            PageReferenceResult {
                section_number: reference.section_number.clone(),
                issue,
            }
        })
        .collect()
}

fn issue(code: &str, message: String) -> ValidationIssue {
    ValidationIssue {
        code: code.to_owned(),
        blocking: true,
        message,
    }
}
fn normalize(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}
fn heading_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"^\s*(\d+(?:\.\d+){0,3})\.?\s+(.+?)\s*$").unwrap())
}
fn section_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?i)^SECTION\s+[A-Z]+$").unwrap())
}
fn toc_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"^\s*(\d+(?:\.\d+){0,3})\.?\s+(.+?)\s+(\d+)\s*$").unwrap())
}
