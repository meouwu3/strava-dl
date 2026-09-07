use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Folder,
    Zip,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Media {
    pub url: String,
    pub kind: String,
    pub source: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
    #[serde(skip)]
    pub candidates: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ArchivePlan {
    pub athlete_id: String,
    pub output: Output,
    pub output_dir: PathBuf,
    pub items: Vec<Media>,
}

impl Media {
    pub fn with_candidates(
        kind: &str,
        urls: Vec<String>,
        source: String,
        filename: String,
        activity_id: Option<String>,
    ) -> Self {
        let url = urls
            .first()
            .cloned()
            .unwrap_or_else(|| "media.bin".to_string());
        Self {
            url,
            kind: kind.into(),
            source,
            filename,
            activity_id,
            candidates: urls,
        }
    }

    pub fn download_urls(&self) -> Vec<String> {
        if self.candidates.is_empty() {
            vec![self.url.clone()]
        } else {
            self.candidates.clone()
        }
    }
}
