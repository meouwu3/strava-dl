use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::client::{looks_like_media, Fetcher};
use crate::hls;
use crate::model::{ArchivePlan, Media, Output};

pub fn write_archive(fetcher: &dyn Fetcher, plan: &ArchivePlan) -> Result<PathBuf> {
    let root = plan.output_dir.join(format!("strava-{}", plan.athlete_id));
    if root.exists() {
        fs::remove_dir_all(&root).with_context(|| format!("remove {}", root.display()))?;
    }
    fs::create_dir_all(&root).with_context(|| format!("create {}", root.display()))?;

    for (index, item) in plan.items.iter().enumerate() {
        eprint!("\rdownloading {}/{}", index + 1, plan.items.len());
        let bytes = download_item(fetcher, item)?;
        let dest = unique_path(&root, &item.filename);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, bytes).with_context(|| format!("write {}", dest.display()))?;
    }
    if !plan.items.is_empty() {
        eprintln!();
    }
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&plan.items)?,
    )?;

    match plan.output {
        Output::Folder => Ok(root),
        Output::Zip => {
            let zip_path = plan
                .output_dir
                .join(format!("strava-{}.zip", plan.athlete_id));
            zip_tree(&root, &zip_path)?;
            fs::remove_dir_all(&root)?;
            Ok(zip_path)
        }
    }
}

fn download_item(fetcher: &dyn Fetcher, item: &Media) -> Result<Vec<u8>> {
    let mut last_error = None;
    let mut best: Option<Vec<u8>> = None;
    for url in item.download_urls() {
        let result = if item.kind == "video" && url.contains("m3u8") {
            hls::download(fetcher, &url)
        } else {
            fetcher.get_bytes(&url)
        };
        match result {
            Ok(bytes)
                if looks_like_media(&bytes) || (item.kind == "video" && !bytes.is_empty()) =>
            {
                let better = best.as_ref().map(|b| bytes.len() > b.len()).unwrap_or(true);
                if better {
                    best = Some(bytes);
                }
                let n = best.as_ref().map(Vec::len).unwrap_or(0);
                if item.kind == "video" && n > 100_000 {
                    break;
                }
                if item.kind == "image" && n >= 80_000 {
                    break;
                }
            }
            Ok(_) => last_error = Some(anyhow::anyhow!("response for {url} was not media")),
            Err(err) => last_error = Some(err),
        }
    }
    best.ok_or_else(|| {
        last_error.unwrap_or_else(|| anyhow::anyhow!("no download URL for {}", item.url))
    })
}

fn unique_path(root: &Path, filename: &str) -> PathBuf {
    let dest = root.join(filename);
    if !dest.exists() {
        return dest;
    }
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = dest
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    let parent = dest.parent().unwrap_or(root);
    let mut n = 2;
    loop {
        let candidate = parent.join(format!("{stem}-{n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

fn zip_tree(src: &Path, dst: &Path) -> Result<()> {
    let file = File::create(dst).with_context(|| format!("create {}", dst.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir(&mut zip, src, src, options)?;
    zip.finish()?;
    Ok(())
}

fn add_dir(
    zip: &mut ZipWriter<File>,
    root: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(root)?;
        let name = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), options)?;
            add_dir(zip, root, &path, options)?;
        } else {
            zip.start_file(&name, options)?;
            let mut f = File::open(&path)?;
            let mut data = Vec::new();
            f.read_to_end(&mut data)?;
            zip.write_all(&data)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Fetcher;
    use anyhow::Result;
    use std::collections::HashMap;

    struct MapFetcher {
        bytes: HashMap<String, Vec<u8>>,
    }

    impl Fetcher for MapFetcher {
        fn get_text(&self, url: &str, _xhr: bool) -> Result<String> {
            anyhow::bail!("no text {url}")
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
            self.bytes
                .get(url)
                .cloned()
                .with_context(|| format!("missing {url}"))
        }
    }

    #[test]
    fn writes_folder_manifest_and_zip() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg = [0xFFu8, 0xD8, 0xFF, 0xE0]
            .into_iter()
            .chain([0u8; 32])
            .collect::<Vec<_>>();
        let fetcher = MapFetcher {
            bytes: HashMap::from([(
                "https://dgtzuqphqg23d.cloudfront.net/a.jpg".into(),
                jpeg.clone(),
            )]),
        };
        let plan = ArchivePlan {
            athlete_id: "1".into(),
            output: Output::Folder,
            output_dir: dir.path().to_path_buf(),
            items: vec![Media::with_candidates(
                "image",
                vec!["https://dgtzuqphqg23d.cloudfront.net/a.jpg".into()],
                "https://www.strava.com/activities/1".into(),
                "1/photo-01.jpg".into(),
                Some("1".into()),
            )],
        };
        let root = write_archive(&fetcher, &plan).unwrap();
        assert!(root.join("1/photo-01.jpg").exists());
        assert!(root.join("manifest.json").exists());

        let mut zipped = plan.clone();
        zipped.output = Output::Zip;
        zipped.output_dir = dir.path().to_path_buf();
        let zip_path = write_archive(&fetcher, &zipped).unwrap();
        assert!(zip_path.ends_with("strava-1.zip"));
        assert!(zip_path.exists());
    }

    #[test]
    fn prefers_largest_successful_photo() {
        let dir = tempfile::tempdir().unwrap();
        let thumb = [0xFFu8, 0xD8, 0xFF, 0xE0]
            .into_iter()
            .chain([1u8; 32])
            .collect::<Vec<_>>();
        let large = [0xFFu8, 0xD8, 0xFF, 0xE0]
            .into_iter()
            .chain([2u8; 90_000])
            .collect::<Vec<_>>();
        let fetcher = MapFetcher {
            bytes: HashMap::from([
                (
                    "https://dgtzuqphqg23d.cloudfront.net/a-128x85.jpg".into(),
                    thumb,
                ),
                (
                    "https://dgtzuqphqg23d.cloudfront.net/a-2048x1365.jpg".into(),
                    large,
                ),
            ]),
        };
        let plan = ArchivePlan {
            athlete_id: "1".into(),
            output: Output::Folder,
            output_dir: dir.path().to_path_buf(),
            items: vec![Media::with_candidates(
                "image",
                vec![
                    "https://dgtzuqphqg23d.cloudfront.net/a-2048x1365.jpg".into(),
                    "https://dgtzuqphqg23d.cloudfront.net/a-128x85.jpg".into(),
                ],
                "https://www.strava.com/activities/1".into(),
                "1/photo-01.jpg".into(),
                Some("1".into()),
            )],
        };
        let root = write_archive(&fetcher, &plan).unwrap();
        let bytes = std::fs::read(root.join("1/photo-01.jpg")).unwrap();
        assert!(bytes.len() > 80_000);
        assert_eq!(bytes[4], 2);
    }
}
