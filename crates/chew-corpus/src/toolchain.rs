use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::CorpusError;

#[derive(Debug, Clone)]
pub struct PopplerToolchain {
    pdfinfo: PathBuf,
    #[allow(dead_code)]
    pdftotext: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfMetadata {
    pub pages: u32,
    pub encrypted: bool,
    pub file_size: u64,
    pub pdf_version: String,
}

impl PopplerToolchain {
    pub fn probe(pdfinfo: PathBuf, pdftotext: PathBuf) -> Result<Self, CorpusError> {
        let info = run(&pdfinfo, &["-v"])?;
        let text = run(&pdftotext, &["-v"])?;
        let info_version = version(&info)?;
        let text_version = version(&text)?;
        if info_version != text_version {
            return Err(CorpusError::ToolOutput {
                field: "matching Poppler versions",
            });
        }
        Ok(Self {
            pdfinfo,
            pdftotext,
            version: info_version,
        })
    }

    pub fn pdf_metadata(&self, path: impl AsRef<Path>) -> Result<PdfMetadata, CorpusError> {
        let path = path.as_ref().to_string_lossy().into_owned();
        let output = run(&self.pdfinfo, &[&path])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(PdfMetadata {
            pages: value(&stdout, "Pages:")?
                .parse()
                .map_err(|_| CorpusError::ToolOutput { field: "Pages" })?,
            encrypted: value(&stdout, "Encrypted:")? != "no",
            file_size: value(&stdout, "File size:")?
                .split_whitespace()
                .next()
                .ok_or(CorpusError::ToolOutput { field: "File size" })?
                .parse()
                .map_err(|_| CorpusError::ToolOutput { field: "File size" })?,
            pdf_version: value(&stdout, "PDF version:")?.to_owned(),
        })
    }
}

fn run(program: &Path, args: &[&str]) -> Result<Output, CorpusError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| CorpusError::Read {
            path: program.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(CorpusError::Command {
            command: program.display().to_string(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(4096)
                .collect(),
        });
    }
    Ok(output)
}

fn version(output: &Output) -> Result<String, CorpusError> {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    combined
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or(CorpusError::ToolOutput { field: "version" })
}

fn value<'a>(output: &'a str, label: &str) -> Result<&'a str, CorpusError> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(label).map(str::trim))
        .ok_or(CorpusError::ToolOutput {
            field: "metadata field",
        })
}
