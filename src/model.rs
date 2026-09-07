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
}

#[derive(Clone, Debug)]
pub struct ArchivePlan {
    pub athlete_id: String,
    pub output: Output,
    pub output_dir: PathBuf,
    pub items: Vec<Media>,
}

impl Media {
    pub fn image(
        url: String,
        source: String,
        filename: String,
        activity_id: Option<String>,
    ) -> Self {
        Self {
            url,
            kind: "image".into(),
            source,
            filename,
            activity_id,
        }
    }

    pub fn video(
        url: String,
        source: String,
        filename: String,
        activity_id: Option<String>,
    ) -> Self {
        Self {
            url,
            kind: "video".into(),
            source,
            filename,
            activity_id,
        }
    }
}
