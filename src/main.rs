use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipWriter};

#[derive(Parser, Debug)]
#[command(
    name = "strava-dl",
    version,
    about = "Download accessible Strava athlete media"
)]
struct Args {
    athlete_id: String,
    #[arg(long, value_enum, default_value_t = Output::Folder)]
    output: Output,
    #[arg(long, default_value_os_t = downloads_dir())]
    output_dir: PathBuf,
    #[arg(long, env = "STRAVA_COOKIE", hide_env_values = true)]
    cookie: Option<String>,
    #[arg(long, env = "STRAVA_ACCESS_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[arg(long, default_value_t = 30)]
    timeout: u64,
}
#[derive(Clone, Debug, ValueEnum)]
enum Output {
    Folder,
    Zip,
}
#[derive(Serialize)]
struct Media {
    url: String,
    kind: String,
    source: String,
    filename: String,
}

fn downloads_dir() -> PathBuf {
    dirs_fallback()
}
fn dirs_fallback() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Downloads")
}

fn main() -> Result<()> {
    let args = Args::parse();
    if !args.athlete_id.chars().all(|c| c.is_ascii_digit()) {
        bail!("athlete id must contain only digits")
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(args.timeout))
        .user_agent("strava-media-archive/0.2")
        .build()?;
    let mut request = client.get(format!(
        "https://www.strava.com/athletes/{}",
        args.athlete_id
    ));
    if let Some(cookie) = &args.cookie {
        request = request.header("cookie", cookie);
    }
    if let Some(token) = &args.token {
        request = request.bearer_auth(token);
    }
    let html = request.send()?.error_for_status()?.text()?;
    let re = Regex::new(r#"https?://[^\"'<> ]+"#)?;
    let mut seen = HashSet::new();
    let mut media = Vec::new();
    for raw in re.find_iter(&html) {
        let url = raw.as_str().replace("\\u0026", "&").replace("\\/", "/");
        let lower = url.to_lowercase();
        if !lower.contains("cloudfront.net/pictures/") && !lower.contains("cloudfront.net/media/") {
            continue;
        }
        let kind = if [".mp4", ".mov", ".webm"].iter().any(|x| lower.contains(x)) {
            "video"
        } else if [".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic"]
            .iter()
            .any(|x| lower.contains(x))
        {
            "image"
        } else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }
        let filename = url
            .split('/')
            .last()
            .unwrap_or("media.bin")
            .split('?')
            .next()
            .unwrap_or("media.bin")
            .to_string();
        media.push(Media {
            url,
            kind: kind.into(),
            source: format!("https://www.strava.com/athletes/{}", args.athlete_id),
            filename,
        });
    }
    if media.is_empty() {
        bail!("no accessible media found; Strava privacy or login may be required")
    }
    let root = args.output_dir.join(format!("strava-{}", args.athlete_id));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    for (i, item) in media.iter().enumerate() {
        let name = if root.join(&item.filename).exists() {
            format!("{:04}-{}", i, item.filename)
        } else {
            item.filename.clone()
        };
        let bytes = client.get(&item.url).send()?.error_for_status()?.bytes()?;
        fs::write(root.join(name), bytes)?;
    }
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&media)?,
    )?;
    match args.output {
        Output::Folder => println!("{}", root.display()),
        Output::Zip => {
            let zip_path = args
                .output_dir
                .join(format!("strava-{}.zip", args.athlete_id));
            zip_dir(&root, &zip_path)?;
            fs::remove_dir_all(root)?;
            println!("{}", zip_path.display());
        }
    }
    eprintln!("archived {} media files", media.len());
    Ok(())
}
fn zip_dir(src: &Path, dst: &Path) -> Result<()> {
    let file = File::create(dst)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for entry in fs::read_dir(src)? {
        let path = entry?.path();
        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy();
            zip.start_file(name.as_ref(), options)?;
            let mut f = File::open(path)?;
            let mut data = Vec::new();
            f.read_to_end(&mut data)?;
            zip.write_all(&data)?;
        }
    }
    zip.finish()?;
    Ok(())
}
