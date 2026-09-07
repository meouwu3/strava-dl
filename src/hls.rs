use anyhow::{bail, Context, Result};
use url::Url;

use crate::client::Fetcher;

pub fn download(fetcher: &dyn Fetcher, playlist_url: &str) -> Result<Vec<u8>> {
    let master = fetcher.get_text(playlist_url, false)?;
    let selected = select_playlist(&master, playlist_url)?;
    let media = if selected == playlist_url {
        master
    } else {
        fetcher.get_text(&selected, false)?
    };
    let segments = media_segments(&media, &selected)?;
    if segments.is_empty() {
        bail!("HLS playlist contained no segments: {playlist_url}");
    }
    let mut out = Vec::new();
    for segment in segments {
        out.extend(fetcher.get_bytes(&segment)?);
    }
    Ok(out)
}

fn select_playlist(contents: &str, playlist_url: &str) -> Result<String> {
    let mut best: Option<(u128, String)> = None;
    let mut pending_score = None;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.starts_with("#EXT-X-I-FRAME-STREAM-INF:") {
            pending_score = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            pending_score = Some(stream_score(rest));
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            pending_score = None;
            continue;
        }
        if let Some(score) = pending_score.take() {
            let resolved = resolve(playlist_url, line)?;
            if best.as_ref().map(|(n, _)| *n).unwrap_or(0) <= score {
                best = Some((score, resolved));
            }
        }
    }
    Ok(best
        .map(|(_, url)| url)
        .unwrap_or_else(|| playlist_url.to_string()))
}

fn stream_score(attrs: &str) -> u128 {
    let pixels = parse_resolution_pixels(attrs).unwrap_or(0) as u128;
    let bandwidth = parse_attr_u64(attrs, "BANDWIDTH").unwrap_or(0) as u128;
    (pixels << 32) | bandwidth
}

fn parse_resolution_pixels(attrs: &str) -> Option<u64> {
    let value = attr_value(attrs, "RESOLUTION")?;
    let (w, h) = value.split_once('x')?;
    Some(w.parse::<u64>().ok()?.saturating_mul(h.parse().ok()?))
}

fn parse_attr_u64(attrs: &str, key: &str) -> Option<u64> {
    attr_value(attrs, key)?.parse().ok()
}

fn attr_value<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let lower = attrs.to_ascii_lowercase();
    let needle = format!("{}=", key.to_ascii_lowercase());
    let idx = lower.find(&needle)?;
    let rest = &attrs[idx + needle.len()..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(&stripped[..end])
    } else {
        Some(rest.split(',').next()?.trim())
    }
}

fn media_segments(contents: &str, playlist_url: &str) -> Result<Vec<String>> {
    let mut segments = Vec::new();
    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        segments.push(resolve(playlist_url, line)?);
    }
    Ok(segments)
}

fn resolve(base: &str, href: &str) -> Result<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(href.to_string());
    }
    Url::parse(base)
        .with_context(|| format!("invalid playlist url {base}"))?
        .join(href)
        .map(|u| u.to_string())
        .with_context(|| format!("invalid HLS href {href}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Fetcher;
    use anyhow::Result;
    use std::collections::HashMap;

    struct MapFetcher {
        text: HashMap<String, String>,
        bytes: HashMap<String, Vec<u8>>,
    }

    impl Fetcher for MapFetcher {
        fn get_text(&self, url: &str, _xhr: bool) -> Result<String> {
            self.text
                .get(url)
                .cloned()
                .with_context(|| format!("missing text {url}"))
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
            self.bytes
                .get(url)
                .cloned()
                .with_context(|| format!("missing bytes {url}"))
        }
    }

    #[test]
    fn picks_highest_bandwidth_and_concats_segments() {
        let master = "https://cdn.example/master.m3u8";
        let high = "https://cdn.example/high.m3u8";
        let fetcher = MapFetcher {
            text: HashMap::from([
                (
                    master.into(),
                    "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=5000\nhigh.m3u8\n"
                        .into(),
                ),
                (
                    high.into(),
                    "#EXTM3U\n#EXTINF:1,\nseg0.ts\n#EXTINF:1,\nhttps://cdn.example/seg1.ts\n".into(),
                ),
            ]),
            bytes: HashMap::from([
                ("https://cdn.example/seg0.ts".into(), b"AAA".to_vec()),
                ("https://cdn.example/seg1.ts".into(), b"BBB".to_vec()),
            ]),
        };
        let out = download(&fetcher, master).unwrap();
        assert_eq!(out, b"AAABBB");
    }

    #[test]
    fn prefers_resolution_over_bandwidth() {
        let master = concat!(
            "#EXTM3U\n",
            "#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=90000000,RESOLUTION=1920x1080,URI=\"iframes.m3u8\"\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=8000000,RESOLUTION=1280x720\n",
            "720.m3u8\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=5000000,RESOLUTION=1920x1080\n",
            "1080.m3u8\n"
        );
        assert_eq!(
            select_playlist(master, "https://cdn.example/master.m3u8").unwrap(),
            "https://cdn.example/1080.m3u8"
        );
    }
}
