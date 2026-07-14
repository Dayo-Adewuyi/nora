use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceCadre {
    #[serde(rename = "JCHEW")]
    Jchew,
    #[serde(rename = "CHEW")]
    Chew,
    #[serde(rename = "CHO")]
    Cho,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    Text,
    Illustrated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub source_id: String,
    pub title: String,
    pub cadre: SourceCadre,
    pub edition: String,
    pub representation: Representation,
    pub official_url: String,
    pub retrieved_at: String,
    pub byte_size: u64,
    pub sha256: String,
    pub page_count: u32,
    pub local_path: PathBuf,
}
