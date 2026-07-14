use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{CorpusError, Representation, SourceCadre, SourceManifest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePair {
    pub cadre: SourceCadre,
    pub text: PathBuf,
    pub illustrated: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub schema_version: u32,
    pub derived_path: PathBuf,
    pub pairs: Vec<SourcePair>,
}

impl PipelineConfig {
    pub fn load(repo_root: &Path, config_path: &Path) -> Result<Self, CorpusError> {
        let path = repo_root.join(config_path);
        let bytes = fs::read(&path).map_err(|source| CorpusError::Read {
            path: path.clone(),
            source,
        })?;
        let config: Self = serde_json::from_slice(&bytes).map_err(|source| CorpusError::Json {
            path: path.clone(),
            source,
        })?;
        config.validate(repo_root)?;
        Ok(config)
    }

    fn validate(&self, repo_root: &Path) -> Result<(), CorpusError> {
        if self.schema_version != 1 {
            return Err(CorpusError::InvalidConfig(
                "schema_version must be 1".into(),
            ));
        }
        let mut cadres = HashSet::new();
        let mut paths = HashSet::new();
        for pair in &self.pairs {
            if !cadres.insert(pair.cadre) {
                return Err(CorpusError::InvalidConfig("duplicate cadre pair".into()));
            }
            if !paths.insert(&pair.text) || !paths.insert(&pair.illustrated) {
                return Err(CorpusError::InvalidConfig("manifest path reused".into()));
            }
            let text = load_manifest(repo_root, &pair.text)?;
            let illustrated = load_manifest(repo_root, &pair.illustrated)?;
            if text.cadre != pair.cadre || illustrated.cadre != pair.cadre {
                return Err(CorpusError::InvalidConfig("manifest cadre mismatch".into()));
            }
            if text.representation != Representation::Text
                || illustrated.representation != Representation::Illustrated
            {
                return Err(CorpusError::InvalidConfig(
                    "invalid representation pair".into(),
                ));
            }
        }
        Ok(())
    }
}

fn load_manifest(repo_root: &Path, manifest_path: &Path) -> Result<SourceManifest, CorpusError> {
    let path = repo_root.join(manifest_path);
    let bytes = fs::read(&path).map_err(|source| CorpusError::Read {
        path: path.clone(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| CorpusError::Json { path, source })
}
