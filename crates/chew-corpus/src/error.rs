use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid pipeline configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid source manifest: {0}")]
    InvalidManifest(String),
    #[error("byte size mismatch for {path}: expected {expected}, got {actual}")]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("command {command} failed with status {status}: {stderr}")]
    Command {
        command: String,
        status: String,
        stderr: String,
    },
    #[error("could not parse {field} from tool output")]
    ToolOutput { field: &'static str },
    #[error("invalid extraction output: {0}")]
    InvalidExtraction(String),
}
