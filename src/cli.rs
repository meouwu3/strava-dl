use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use crate::model::Output;

#[derive(Parser, Debug)]
#[command(
    name = "strava-dl",
    version,
    about = "Download accessible Strava athlete media"
)]
pub struct Args {
    pub athlete_id: String,
    #[arg(long, value_enum, default_value_t = OutputArg::Folder)]
    pub output: OutputArg,
    #[arg(long, default_value_os_t = downloads_dir())]
    pub output_dir: PathBuf,
    #[arg(long, env = "STRAVA_COOKIE", hide_env_values = true)]
    pub cookie: Option<String>,
    #[arg(long, env = "STRAVA_ACCESS_TOKEN", hide_env_values = true)]
    pub token: Option<String>,
    #[arg(long, env = "STRAVA_CSRF_TOKEN", hide_env_values = true)]
    pub csrf: Option<String>,
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,
    #[arg(
        long,
        env = "STRAVA_BASE_URL",
        default_value = "https://www.strava.com",
        hide = true
    )]
    pub base_url: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputArg {
    Folder,
    Zip,
}

impl From<OutputArg> for Output {
    fn from(value: OutputArg) -> Self {
        match value {
            OutputArg::Folder => Output::Folder,
            OutputArg::Zip => Output::Zip,
        }
    }
}

pub fn downloads_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Downloads")
    })
}
