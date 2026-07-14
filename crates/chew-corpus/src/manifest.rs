use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::CorpusError;

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

#[derive(Debug)]
pub struct VerifiedSource {
    pub manifest: SourceManifest,
    pub absolute_path: PathBuf,
    pub verified_sha256: String,
}

impl SourceManifest {
    pub fn verify(&self, repo_root: &Path) -> Result<VerifiedSource, CorpusError> {
        self.validate_fields()?;
        let path = repo_root.join(&self.local_path);
        let file = File::open(&path).map_err(|source| CorpusError::Read {
            path: path.clone(),
            source,
        })?;
        let actual_size = file
            .metadata()
            .map_err(|source| CorpusError::Read {
                path: path.clone(),
                source,
            })?
            .len();
        if actual_size != self.byte_size {
            return Err(CorpusError::SizeMismatch {
                path,
                expected: self.byte_size,
                actual: actual_size,
            });
        }
        let mut reader = BufReader::with_capacity(64 * 1024, file);
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|source| CorpusError::Read {
                    path: path.clone(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let actual = hex::encode(hasher.finalize());
        if actual != self.sha256 {
            return Err(CorpusError::ChecksumMismatch {
                path,
                expected: self.sha256.clone(),
                actual,
            });
        }
        Ok(VerifiedSource {
            manifest: self.clone(),
            absolute_path: path,
            verified_sha256: actual,
        })
    }

    fn validate_fields(&self) -> Result<(), CorpusError> {
        if !self.official_url.starts_with("https://chprbn.gov.ng/") {
            return Err(CorpusError::InvalidManifest(
                "official_url must use CHPRBN HTTPS".into(),
            ));
        }
        if self.local_path.is_absolute()
            || self
                .local_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(CorpusError::InvalidManifest(
                "local_path must stay within the repository".into(),
            ));
        }
        let date = self.retrieved_at.as_bytes();
        if date.len() != 10 || date[4] != b'-' || date[7] != b'-' || self.page_count == 0 {
            return Err(CorpusError::InvalidManifest(
                "invalid retrieval date or page count".into(),
            ));
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(CorpusError::InvalidManifest(
                "sha256 must be 64 hexadecimal characters".into(),
            ));
        }
        Ok(())
    }
}
