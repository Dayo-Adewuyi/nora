use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{BoundingPage, CorpusError, TextBlock};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionWarning {
    MissingPrintedPageLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRecord {
    pub schema_version: u32,
    pub source_id: String,
    pub physical_page: u32,
    pub printed_page_label: Option<String>,
    pub width: f32,
    pub height: f32,
    pub layout_text: String,
    pub blocks: Vec<TextBlock>,
    pub header_candidates: Vec<String>,
    pub footer_candidates: Vec<String>,
    pub normalized_content_hash: String,
    pub warnings: Vec<ExtractionWarning>,
}

pub fn build_page_records(
    source_id: &str,
    layout: &str,
    bounding: Vec<BoundingPage>,
) -> Result<Vec<PageRecord>, CorpusError> {
    let mut layouts: Vec<&str> = layout.split('\x0c').collect();
    if layouts.last().is_some_and(|last| last.is_empty()) {
        layouts.pop();
    }
    if layouts.len() != bounding.len() {
        return Err(CorpusError::InvalidExtraction(format!(
            "layout has {} pages but bbox has {}",
            layouts.len(),
            bounding.len()
        )));
    }
    let page_pattern = Regex::new(r"(?i)\bPAGE\s+([0-9]+)\b").expect("valid page regex");
    Ok(layouts
        .into_iter()
        .zip(bounding)
        .enumerate()
        .map(|(index, (text, page))| {
            let printed = page_pattern
                .captures(text)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str().to_owned());
            let warnings = if printed.is_none() {
                vec![ExtractionWarning::MissingPrintedPageLabel]
            } else {
                Vec::new()
            };
            let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
            PageRecord {
                schema_version: 1,
                source_id: source_id.to_owned(),
                physical_page: (index + 1) as u32,
                printed_page_label: printed,
                width: page.width,
                height: page.height,
                layout_text: text.to_owned(),
                blocks: page.blocks,
                header_candidates: text.lines().take(3).map(str::to_owned).collect(),
                footer_candidates: text.lines().rev().take(3).map(str::to_owned).collect(),
                normalized_content_hash: hex::encode(Sha256::digest(normalized.as_bytes())),
                warnings,
            }
        })
        .collect())
}
