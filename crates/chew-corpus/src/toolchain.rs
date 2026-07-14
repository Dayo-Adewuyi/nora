use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::{parse_bbox_layout, BoundingPage, CorpusError};

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

#[derive(Debug)]
pub struct RawExtraction {
    pub layout_pages: Vec<String>,
    pub bounding_pages: Vec<BoundingPage>,
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

    pub fn extract(&self, path: &Path, output_dir: &Path) -> Result<RawExtraction, CorpusError> {
        let metadata = self.pdf_metadata(path)?;
        let layout_path = output_dir.join("layout.txt");
        let bbox_path = output_dir.join("bbox.xhtml");
        let first = "1".to_owned();
        let last = metadata.pages.to_string();
        let pdf = path.to_string_lossy().into_owned();
        let layout = layout_path.to_string_lossy().into_owned();
        let bbox = bbox_path.to_string_lossy().into_owned();
        run(
            &self.pdftotext,
            &[
                "-f", &first, "-l", &last, "-layout", "-enc", "UTF-8", "-eol", "unix", &pdf,
                &layout,
            ],
        )?;
        run(
            &self.pdftotext,
            &[
                "-f",
                &first,
                "-l",
                &last,
                "-bbox-layout",
                "-enc",
                "UTF-8",
                "-eol",
                "unix",
                &pdf,
                &bbox,
            ],
        )?;
        let layout_text = fs::read_to_string(&layout_path).map_err(|source| CorpusError::Read {
            path: layout_path,
            source,
        })?;
        let mut layout_pages: Vec<String> = layout_text.split('\x0c').map(str::to_owned).collect();
        if layout_pages.last().is_some_and(String::is_empty) {
            layout_pages.pop();
        }
        let bbox_file = fs::File::open(&bbox_path).map_err(|source| CorpusError::Read {
            path: bbox_path,
            source,
        })?;
        let bounding_pages = parse_bbox_layout(bbox_file)?;
        if layout_pages.len() != metadata.pages as usize
            || bounding_pages.len() != metadata.pages as usize
        {
            return Err(CorpusError::InvalidExtraction(
                "Poppler page count mismatch".into(),
            ));
        }
        Ok(RawExtraction {
            layout_pages,
            bounding_pages,
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
