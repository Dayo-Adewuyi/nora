use std::io::{BufReader, Read};

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use serde::{Deserialize, Serialize};

use crate::CorpusError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub words: Vec<Word>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub lines: Vec<TextLine>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingPage {
    pub width: f32,
    pub height: f32,
    pub blocks: Vec<TextBlock>,
}

pub fn parse_bbox_layout(input: impl Read) -> Result<Vec<BoundingPage>, CorpusError> {
    let mut reader = Reader::from_reader(BufReader::new(input));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut pages = Vec::new();
    let mut page: Option<BoundingPage> = None;
    let mut block: Option<TextBlock> = None;
    let mut line: Option<TextLine> = None;
    let mut word: Option<Word> = None;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| CorpusError::InvalidExtraction(error.to_string()))?
        {
            Event::Start(start) if start.name().as_ref() == b"page" => {
                page = Some(BoundingPage {
                    width: number(&start, b"width")?,
                    height: number(&start, b"height")?,
                    blocks: Vec::new(),
                })
            }
            Event::Start(start) if start.name().as_ref() == b"block" => {
                let (x_min, y_min, x_max, y_max) = rect(&start)?;
                block = Some(TextBlock {
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    lines: Vec::new(),
                });
            }
            Event::Start(start) if start.name().as_ref() == b"line" => {
                let (x_min, y_min, x_max, y_max) = rect(&start)?;
                line = Some(TextLine {
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    words: Vec::new(),
                });
            }
            Event::Start(start) if start.name().as_ref() == b"word" => {
                let (x_min, y_min, x_max, y_max) = rect(&start)?;
                word = Some(Word {
                    text: String::new(),
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                });
            }
            Event::Text(text) if word.is_some() => word.as_mut().unwrap().text.push_str(
                &text
                    .decode()
                    .map_err(|error| CorpusError::InvalidExtraction(error.to_string()))?,
            ),
            Event::End(end) if end.name().as_ref() == b"word" => line
                .as_mut()
                .ok_or_else(|| CorpusError::InvalidExtraction("word outside line".into()))?
                .words
                .push(
                    word.take()
                        .ok_or_else(|| CorpusError::InvalidExtraction("missing word".into()))?,
                ),
            Event::End(end) if end.name().as_ref() == b"line" => block
                .as_mut()
                .ok_or_else(|| CorpusError::InvalidExtraction("line outside block".into()))?
                .lines
                .push(
                    line.take()
                        .ok_or_else(|| CorpusError::InvalidExtraction("missing line".into()))?,
                ),
            Event::End(end) if end.name().as_ref() == b"block" => page
                .as_mut()
                .ok_or_else(|| CorpusError::InvalidExtraction("block outside page".into()))?
                .blocks
                .push(
                    block
                        .take()
                        .ok_or_else(|| CorpusError::InvalidExtraction("missing block".into()))?,
                ),
            Event::End(end) if end.name().as_ref() == b"page" => pages.push(
                page.take()
                    .ok_or_else(|| CorpusError::InvalidExtraction("missing page".into()))?,
            ),
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if pages.is_empty() {
        return Err(CorpusError::InvalidExtraction("no pages".into()));
    }
    Ok(pages)
}

fn rect(start: &BytesStart<'_>) -> Result<(f32, f32, f32, f32), CorpusError> {
    let result = (
        number(start, b"xMin")?,
        number(start, b"yMin")?,
        number(start, b"xMax")?,
        number(start, b"yMax")?,
    );
    if result.2 < result.0 || result.3 < result.1 {
        return Err(CorpusError::InvalidExtraction(
            "invalid bounding box".into(),
        ));
    }
    Ok(result)
}

fn number(start: &BytesStart<'_>, key: &[u8]) -> Result<f32, CorpusError> {
    let value = start
        .attributes()
        .with_checks(false)
        .find_map(|attribute| {
            attribute
                .ok()
                .filter(|item| item.key.as_ref() == key)
                .map(|item| item.value.into_owned())
        })
        .ok_or_else(|| {
            CorpusError::InvalidExtraction(format!(
                "missing attribute {}",
                String::from_utf8_lossy(key)
            ))
        })?;
    let parsed = String::from_utf8(value)
        .map_err(|error| CorpusError::InvalidExtraction(error.to_string()))?
        .parse::<f32>()
        .map_err(|error| CorpusError::InvalidExtraction(error.to_string()))?;
    if !parsed.is_finite() {
        return Err(CorpusError::InvalidExtraction("invalid coordinate".into()));
    }
    Ok(parsed)
}
