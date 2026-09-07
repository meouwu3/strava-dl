#![allow(dead_code)]

use crate::model::Media;
use serde_json::Value;
use std::collections::HashSet;

const STATIC_HOSTS: &[&str] = &["d3nn82uaxijpm6.cloudfront.net", "web-assets.strava.com"];
const AVATAR_HOSTS: &[&str] = &["dgalywyr863hv.cloudfront.net"];
const VIDEO_HOSTS: &[&str] = &["d35tn3x5zm6xrc.cloudfront.net"];

pub fn is_athlete_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
}

pub fn is_login_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("log in | strava") || lower.contains("login | strava")
}

pub fn unescape_url(raw: &str) -> String {
    raw.replace("\\u0026", "&")
        .replace("\\/", "/")
        .replace("&amp;", "&")
}

pub fn original_photo_url(url: &str) -> String {
    let unescaped = unescape_url(url);
    let (base, query) = unescaped
        .split_once('?')
        .map(|(b, q)| (b, Some(q)))
        .unwrap_or((unescaped.as_str(), None));
    let rewritten = if let Some(idx) = base.rfind('/') {
        let (prefix, name) = base.split_at(idx + 1);
        format!("{prefix}{}", regex_lite_strip_size(name))
    } else {
        regex_lite_strip_size(base)
    };
    match query {
        Some(q) if !q.is_empty() => format!("{rewritten}?{q}"),
        _ => rewritten,
    }
}

fn regex_lite_strip_size(name: &str) -> String {
    let bytes = name.as_bytes();
    if let Some(dot) = name.rfind('.') {
        let stem = &name[..dot];
        let ext = &name[dot..];
        if let Some(dash) = stem.rfind('-') {
            let suffix = &stem[dash + 1..];
            if is_size_suffix(suffix) {
                return format!("{}{ext}", &stem[..dash]);
            }
        }
        return name.to_string();
    }
    if let Some(dash) = name.rfind('-') {
        if is_size_suffix(&name[dash + 1..]) {
            return name[..dash].to_string();
        }
    }
    let _ = bytes;
    name.to_string()
}

fn is_size_suffix(value: &str) -> bool {
    let mut parts = value.split('x');
    let Some(w) = parts.next() else {
        return false;
    };
    let Some(h) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !w.is_empty()
        && !h.is_empty()
        && w.chars().all(|c| c.is_ascii_digit())
        && h.chars().all(|c| c.is_ascii_digit())
}

pub fn activity_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    let bytes = text.as_bytes();
    let needle = b"/activities/";
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if bytes[i..].starts_with(needle) {
            let start = i + needle.len();
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                let id = text[start..end].to_string();
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    ids
}

pub fn is_athlete_avatar_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(&unescape_url(url)) else {
        return url.contains("/pictures/athletes/");
    };
    let host = parsed.host_str().unwrap_or_default();
    if AVATAR_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h)) {
        return true;
    }
    parsed
        .path()
        .to_ascii_lowercase()
        .contains("/pictures/athletes/")
}

fn is_video_poster_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(&unescape_url(url)) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or_default();
    if !VIDEO_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h)) {
        return false;
    }
    let path = parsed.path().to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".gif", ".webp"]
        .iter()
        .any(|ext| path.ends_with(ext))
}

pub fn photo_quality_score(url: &str) -> u64 {
    size_from_url(url)
        .map(|(w, h)| w.saturating_mul(h))
        .unwrap_or(0)
}

fn size_from_url(url: &str) -> Option<(u64, u64)> {
    let path = unescape_url(url);
    let path = path.split('?').next().unwrap_or(&path);
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let dash = stem.rfind('-')?;
    parse_wxh(&stem[dash + 1..])
}

pub fn highres_photo_urls(url: &str) -> Vec<String> {
    let unescaped = unescape_url(url);
    let Some((w, h)) = size_from_url(&unescaped) else {
        return vec![unescaped];
    };
    if w.max(h) >= 1536 {
        return vec![unescaped];
    }
    let Some((prefix, ext)) = split_photo_prefix_ext(&unescaped) else {
        return vec![unescaped];
    };
    let mut urls = Vec::new();
    let mut seen = HashSet::new();
    for (nw, nh) in long_side_variants(w, h, 2048) {
        let candidate = format!("{prefix}-{nw}x{nh}{ext}");
        if seen.insert(candidate.clone()) {
            urls.push(candidate);
        }
    }
    for (nw, nh) in long_side_variants(w, h, 1024) {
        let candidate = format!("{prefix}-{nw}x{nh}{ext}");
        if seen.insert(candidate.clone()) {
            urls.push(candidate);
        }
    }
    if seen.insert(unescaped.clone()) {
        urls.push(unescaped);
    }
    urls
}

fn split_photo_prefix_ext(url: &str) -> Option<(String, String)> {
    let (base, query) = url
        .split_once('?')
        .map(|(b, q)| (b, Some(q)))
        .unwrap_or((url, None));
    let slash = base.rfind('/')?;
    let (dir, name) = base.split_at(slash + 1);
    let dot = name.rfind('.')?;
    let stem = &name[..dot];
    let ext = &name[dot..];
    let hash = if let Some(dash) = stem.rfind('-') {
        if parse_wxh(&stem[dash + 1..]).is_some() {
            &stem[..dash]
        } else {
            stem
        }
    } else {
        stem
    };
    let mut prefix = String::from(dir);
    prefix.push_str(hash);
    let ext = match query {
        Some(q) if !q.is_empty() => format!("{ext}?{q}"),
        _ => ext.to_string(),
    };
    Some((prefix, ext))
}

fn long_side_variants(w: u64, h: u64, long_side: u64) -> Vec<(u64, u64)> {
    let w = w.max(1);
    let h = h.max(1);
    let landscape = w >= h;
    let (major, minor) = if landscape { (w, h) } else { (h, w) };
    let exact = minor as f64 * long_side as f64 / major as f64;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for delta in 0i32..=16 {
        for sign in [1i32, -1] {
            if delta == 0 && sign < 0 {
                continue;
            }
            let other = (exact + f64::from(delta * sign)).round() as i64;
            if other <= 0 {
                continue;
            }
            let pair = if landscape {
                (long_side, other as u64)
            } else {
                (other as u64, long_side)
            };
            if seen.insert(pair) {
                out.push(pair);
            }
        }
    }
    out
}

fn parse_wxh(value: &str) -> Option<(u64, u64)> {
    let (w, h) = value.split_once('x')?;
    if w.is_empty() || h.is_empty() {
        return None;
    }
    if !w.chars().all(|c| c.is_ascii_digit()) || !h.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((w.parse().ok()?, h.parse().ok()?))
}

pub fn photo_candidates(urls: &[String]) -> Vec<String> {
    let already_large = urls.iter().any(|url| {
        size_from_url(url)
            .map(|(w, h)| w.max(h) >= 1536)
            .unwrap_or(false)
    });
    let mut expanded = Vec::new();
    for url in urls {
        if already_large {
            if size_from_url(url)
                .map(|(w, h)| w.max(h) >= 1536)
                .unwrap_or(false)
            {
                expanded.push(unescape_url(url));
            }
        } else {
            expanded.extend(highres_photo_urls(url));
        }
    }
    rank_urls(&expanded, |url| {
        if is_athlete_avatar_url(url)
            || is_video_poster_url(url)
            || media_kind(url) != Some("image")
        {
            None
        } else {
            Some(photo_quality_score(url))
        }
    })
}

pub fn video_mp4_url(url: &str) -> Option<String> {
    let url = unescape_url(url);
    let parsed = url::Url::parse(&url).ok()?;
    let host = parsed.host_str()?;
    let hash = video_hash(parsed.path())?;
    Some(format!(
        "{scheme}://{host}/{hash}/{hash}.mp4",
        scheme = parsed.scheme()
    ))
}

fn video_hash(path: &str) -> Option<&str> {
    let path = path.trim_start_matches('/');
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    if parts.len() >= 2 && parts[1] == "hls" {
        return Some(parts[0]);
    }
    if parts.len() >= 2 {
        let stem = parts[1]
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(parts[1]);
        if stem == parts[0] {
            return Some(parts[0]);
        }
    }
    let stem = parts[0]
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(parts[0]);
    if stem.len() >= 8 {
        Some(stem)
    } else {
        None
    }
}

pub fn video_candidates(urls: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    for url in urls {
        if let Some(mp4) = video_mp4_url(url) {
            expanded.push(mp4);
        }
        expanded.push(unescape_url(url));
    }
    rank_urls(&expanded, |url| {
        if media_kind(url) != Some("video") {
            return None;
        }
        let path = unescape_url(url).to_ascii_lowercase();
        let rank = if [".mp4", ".mov", ".webm"]
            .iter()
            .any(|ext| path.contains(ext))
        {
            3
        } else if path.contains(".m3u8") {
            2
        } else {
            1
        };
        Some(rank)
    })
}

pub fn media_key(url: &str) -> String {
    if media_kind(url) == Some("video") {
        video_mp4_url(url).unwrap_or_else(|| unescape_url(url))
    } else {
        original_photo_url(url)
    }
}

fn rank_urls(urls: &[String], score: impl Fn(&str) -> Option<u64>) -> Vec<String> {
    let mut ranked = Vec::new();
    let mut seen = HashSet::new();
    for raw in urls {
        let url = unescape_url(raw);
        let Some(s) = score(&url) else {
            continue;
        };
        if seen.insert(url.clone()) {
            ranked.push((s, url));
        }
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, url)| url).collect()
}

pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();
    let decoded = unescape_url(text);
    let bytes = decoded.as_bytes();
    let mut i = 0;
    while i + 8 < bytes.len() {
        let https = bytes[i..].starts_with(b"https://");
        let http = bytes[i..].starts_with(b"http://");
        if https || http {
            let start = i;
            i += if https { 8 } else { 7 };
            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_whitespace()
                    || matches!(
                        c,
                        b'"' | b'\''
                            | b'<'
                            | b'>'
                            | b'\\'
                            | b')'
                            | b'('
                            | b'{'
                            | b'}'
                            | b'['
                            | b']'
                    )
                {
                    break;
                }
                i += 1;
            }
            let raw = decoded[start..i]
                .trim_end_matches(['.', ',', ';'])
                .to_string();
            if is_strava_media_url(&raw) && seen.insert(raw.clone()) {
                urls.push(raw);
            }
        } else {
            i += 1;
        }
    }
    urls
}

pub fn is_strava_media_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(&unescape_url(url)) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or_default();
    if STATIC_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h)) {
        return false;
    }
    if is_athlete_avatar_url(url) || is_video_poster_url(url) {
        return false;
    }
    let path = parsed.path().to_ascii_lowercase();
    let host_l = host.to_ascii_lowercase();
    if host_l.contains("cloudfront.net") || host_l.ends_with(".strava.com") {
        return is_media_path(&path);
    }
    false
}

fn is_media_path(path: &str) -> bool {
    const IMAGE: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic"];
    const VIDEO: &[&str] = &[".mp4", ".mov", ".webm", ".m3u8", ".ts"];
    if IMAGE.iter().chain(VIDEO).any(|ext| path.ends_with(ext)) {
        return true;
    }
    path.contains("/pictures/") || path.contains("/media/")
}

pub fn media_kind(url: &str) -> Option<&'static str> {
    let path = unescape_url(url).to_ascii_lowercase();
    if [".mp4", ".mov", ".webm", ".m3u8", ".ts"]
        .iter()
        .any(|ext| path.contains(ext))
    {
        return Some("video");
    }
    if [".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic"]
        .iter()
        .any(|ext| path.contains(ext))
        || path.contains("/pictures/")
        || path.contains("/media/")
    {
        return Some("image");
    }
    None
}

pub fn media_from_activity_html(html: &str, activity_id: &str) -> Vec<Media> {
    let source = format!("https://www.strava.com/activities/{activity_id}");
    media_from_page(html, &source, activity_id)
}

fn media_from_page(html: &str, source: &str, activity_id: &str) -> Vec<Media> {
    let source = source.to_string();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut photo_i = 0u32;
    let mut video_i = 0u32;

    for raw in parse_media_type_entries(html) {
        let Some((kind, urls)) = media_from_entry(&raw) else {
            continue;
        };
        if let Some(item) = named_media(
            kind,
            urls,
            source.clone(),
            activity_id,
            &mut photo_i,
            &mut video_i,
            &mut seen,
        ) {
            items.push(item);
        }
    }

    for url in extract_urls(html) {
        let Some(kind) = media_kind(&url) else {
            continue;
        };
        if let Some(item) = named_media(
            kind,
            vec![url],
            source.clone(),
            activity_id,
            &mut photo_i,
            &mut video_i,
            &mut seen,
        ) {
            items.push(item);
        }
    }
    items
}

fn named_media(
    kind: &str,
    urls: Vec<String>,
    source: String,
    activity_id: &str,
    photo_i: &mut u32,
    video_i: &mut u32,
    seen: &mut HashSet<String>,
) -> Option<Media> {
    let candidates = if kind == "video" {
        video_candidates(&urls)
    } else {
        photo_candidates(&urls)
    };
    let url = candidates.first()?.clone();
    if !seen.insert(media_key(&url)) {
        return None;
    }
    Some(if kind == "video" {
        *video_i += 1;
        let ext = extension(&url).unwrap_or("mp4");
        let filename = format!("{activity_id}/video-{:02}.{ext}", *video_i);
        Media::with_candidates(
            "video",
            candidates,
            source,
            filename,
            Some(activity_id.into()),
        )
    } else {
        *photo_i += 1;
        let ext = extension(&url).unwrap_or("jpg");
        let filename = format!("{activity_id}/photo-{:02}.{ext}", *photo_i);
        Media::with_candidates(
            "image",
            candidates,
            source,
            filename,
            Some(activity_id.into()),
        )
    })
}

fn extension(url: &str) -> Option<&str> {
    let path = url.split('?').next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit('.').next()?;
    if ext.len() <= 4 && ext.chars().all(|c| c.is_ascii_alphanumeric()) && ext != name {
        Some(ext)
    } else {
        None
    }
}

fn media_from_entry(value: &Value) -> Option<(&'static str, Vec<String>)> {
    let media_type = value.get("media_type").and_then(Value::as_u64).unwrap_or(1);
    if media_type == 2 {
        let urls = string_fields(value, &["video_url", "video", "mp4", "url"]);
        if urls.is_empty() {
            return None;
        }
        return Some(("video", urls));
    }
    let mut urls = string_fields(value, &["original", "large", "url", "thumbnail"]);
    if let Some(sized) = value.get("urls") {
        collect_sized_urls(sized, &mut urls);
    }
    if urls.is_empty() {
        return None;
    }
    Some(("image", urls))
}

fn string_fields(value: &Value, keys: &[&str]) -> Vec<String> {
    let mut urls = Vec::new();
    for key in keys {
        if let Some(url) = value
            .get(*key)
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
        {
            urls.push(url.to_string());
        }
    }
    urls
}

fn collect_sized_urls(value: &Value, urls: &mut Vec<String>) {
    match value {
        Value::String(s) if !s.is_empty() => urls.push(s.clone()),
        Value::Object(map) => {
            for item in map.values() {
                if let Some(s) = item.as_str().filter(|s| !s.is_empty()) {
                    urls.push(s.to_string());
                }
            }
        }
        _ => {}
    }
}

fn parse_media_type_entries(html: &str) -> Vec<Value> {
    let decoded = html
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&#34;", "\"");
    let mut cursor = 0;
    while let Some(hit) = decoded[cursor..].find("\"media_type\"") {
        let abs = cursor + hit;
        if let Some(open) = decoded[..abs].rfind('[') {
            if let Some(close) = matching_bracket(&decoded, open, '[', ']') {
                if let Ok(Value::Array(items)) =
                    serde_json::from_str::<Value>(&decoded[open..=close])
                {
                    if items.iter().any(|item| item.get("media_type").is_some()) {
                        return items;
                    }
                }
                cursor = close + 1;
                continue;
            }
        }
        cursor = abs + 1;
    }
    Vec::new()
}

fn matching_bracket(s: &str, open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut depth = 0;
    let mut in_str = false;
    let mut escape = false;
    for (i, c) in s[open..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            in_str = !in_str;
            continue;
        }
        if in_str {
            continue;
        }
        if c == open_ch {
            depth += 1;
        } else if c == close_ch {
            depth -= 1;
            if depth == 0 {
                return Some(open + i);
            }
        }
    }
    None
}

pub fn activity_ids_from_json(value: &Value, athlete_id: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    walk_activity_ids(value, athlete_id, &mut ids, &mut seen);
    ids
}

fn walk_activity_ids(
    value: &Value,
    athlete_id: &str,
    ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            if let Some(id) = json_id(map.get("id")) {
                if id != athlete_id && is_activity_object(map) && seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
            for nested in map.values() {
                walk_activity_ids(nested, athlete_id, ids, seen);
            }
        }
        Value::Array(items) => {
            for nested in items {
                walk_activity_ids(nested, athlete_id, ids, seen);
            }
        }
        Value::String(text) => {
            for id in activity_ids(text) {
                if id != athlete_id && seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
        _ => {}
    }
}

fn json_id(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) if is_athlete_id(s) => Some(s.clone()),
        _ => None,
    }
}

fn is_activity_object(map: &serde_json::Map<String, Value>) -> bool {
    if map.contains_key("firstname") || map.contains_key("profile") {
        return false;
    }
    map.contains_key("sport_type")
        || map.contains_key("start_date")
        || map.contains_key("start_date_local")
        || map.contains_key("total_photo_count")
        || map.contains_key("map")
        || (map.contains_key("name") && map.contains_key("id") && map.contains_key("type"))
}

pub fn photos_from_api(value: &Value, activity_id: &str) -> Vec<Media> {
    let source = format!("https://www.strava.com/activities/{activity_id}");
    let mut items = Vec::new();
    let mut photo_i = 0u32;
    let mut video_i = 0u32;
    let mut seen = HashSet::new();
    let entries = match value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    for entry in entries {
        if let Some(url) = entry.get("video_url").and_then(Value::as_str) {
            if !url.is_empty() {
                if let Some(item) = named_media(
                    "video",
                    vec![url.to_string()],
                    source.clone(),
                    activity_id,
                    &mut photo_i,
                    &mut video_i,
                    &mut seen,
                ) {
                    items.push(item);
                }
            }
        }
        let mut urls = Vec::new();
        collect_sized_urls(entry.get("urls").unwrap_or(&Value::Null), &mut urls);
        if let Some(item) = named_media(
            "image",
            urls,
            source.clone(),
            activity_id,
            &mut photo_i,
            &mut video_i,
            &mut seen,
        ) {
            items.push(item);
        }
    }
    items
}

pub struct GalleryPage {
    pub items: Vec<Media>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub fn parse_gallery_html(html: &str) -> Option<GalleryPage> {
    let idx = html.find("data-react-class=\"MediaThumbnailList\"")?;
    let rest = &html[idx..];
    let key = "data-react-props=\"";
    let start = rest.find(key)? + key.len();
    let end = rest[start..].find('"')?;
    let encoded = &rest[start..start + end];
    let decoded = encoded
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&");
    parse_gallery_json(&decoded)
}

pub fn parse_gallery_json(text: &str) -> Option<GalleryPage> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    gallery_from_value(&value)
}

fn gallery_from_value(value: &Value) -> Option<GalleryPage> {
    let arr = value.get("items")?.as_array()?;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for entry in arr {
        if let Some(item) = media_from_gallery_item(entry) {
            if seen.insert(media_key(&item.url)) {
                items.push(item);
            }
        }
    }
    let next_cursor = value
        .get("nextCursor")
        .or_else(|| value.get("next_cursor"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    let has_more = value
        .get("hasMore")
        .or_else(|| value.get("has_more"))
        .and_then(Value::as_bool)
        .unwrap_or(next_cursor.is_some());
    Some(GalleryPage {
        items,
        next_cursor,
        has_more,
    })
}

fn media_from_gallery_item(item: &Value) -> Option<Media> {
    let media_type = item.get("media_type").and_then(Value::as_u64).unwrap_or(1);
    if media_type == 2 {
        let video = item
            .get("video")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())?;
        let candidates = video_candidates(&[video.to_string()]);
        let url = candidates.first()?.clone();
        return Some(Media::with_candidates(
            "video",
            candidates,
            url.clone(),
            "video.mp4".into(),
            None,
        ));
    }
    let large = item
        .get("large")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| item.get("thumbnail").and_then(Value::as_str))?;
    let mut urls = vec![large.to_string()];
    if let Some(thumb) = item.get("thumbnail").and_then(Value::as_str) {
        urls.push(thumb.to_string());
    }
    let candidates = photo_candidates(&urls);
    let url = candidates.first()?.clone();
    Some(Media::with_candidates(
        "image",
        candidates,
        url.clone(),
        "photo.jpg".into(),
        None,
    ))
}

pub fn assign_flat_names(athlete_id: &str, items: &mut [Media]) {
    for (index, item) in items.iter_mut().enumerate() {
        let ext = if item.kind == "video" {
            extension(&item.url).unwrap_or("mp4")
        } else {
            extension(&item.url).unwrap_or("jpg")
        };
        item.filename = format!("strava-{athlete_id}-{:03}.{ext}", index + 1);
    }
}

pub fn parse_json_or_none(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(trimmed).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn athlete_id_must_be_digits() {
        assert!(is_athlete_id("135539874"));
        assert!(!is_athlete_id("abc"));
        assert!(!is_athlete_id(""));
    }

    #[test]
    fn strips_size_suffix_for_original_photo() {
        assert_eq!(
            original_photo_url("https://dgtzuqphqg23d.cloudfront.net/hash-2048x1152.jpg"),
            "https://dgtzuqphqg23d.cloudfront.net/hash.jpg"
        );
        let candidates = photo_candidates(&[
            "https://dgtzuqphqg23d.cloudfront.net/hash-64x64.jpg".into(),
            "https://dgtzuqphqg23d.cloudfront.net/hash-2048x1152.jpg".into(),
        ]);
        assert!(candidates[0].contains("2048"));
        assert!(candidates
            .iter()
            .any(|u| u.ends_with("/hash-2048x1152.jpg")));
        assert!(!candidates[0].ends_with("/hash.jpg"));
    }

    #[test]
    fn upgrades_thumbnail_to_2048_long_side() {
        let urls = highres_photo_urls(
            "https://dgtzuqphqg23d.cloudfront.net/130IMT0SrbqOQtKxf7UzNnnwDxkB7mLTz9gmfUHlP10-128x85.jpg",
        );
        assert!(urls.iter().any(|u| u.ends_with("-2048x1365.jpg")));
        let portrait = highres_photo_urls(
            "https://dgtzuqphqg23d.cloudfront.net/ceULkD9O5EeCdXeWW1u7YaTEwpuLsxhH6QLNdnTEHLA-96x128.jpg",
        );
        assert!(portrait.iter().any(|u| u.ends_with("-1536x2048.jpg")));
    }

    #[test]
    fn ignores_static_assets_and_avatars() {
        assert!(!is_strava_media_url(
            "https://d3nn82uaxijpm6.cloudfront.net/favicon-32x32.png"
        ));
        assert!(is_strava_media_url(
            "https://dgtzuqphqg23d.cloudfront.net/hash-2048x1152.jpg"
        ));
        assert!(!is_strava_media_url(
            "https://dgalywyr863hv.cloudfront.net/pictures/athletes/1/2/12/medium.jpg"
        ));
        assert!(is_athlete_avatar_url(
            "https://dgalywyr863hv.cloudfront.net/pictures/athletes/1/2/12/medium.jpg"
        ));
    }

    #[test]
    fn extracts_activity_ids_and_media_blob() {
        let html = r#"<div data-react-props="[{&quot;media_type&quot;:1,&quot;large&quot;:&quot;https://dgtzuqphqg23d.cloudfront.net/pic-2048x1152.jpg&quot;,&quot;video&quot;:null},{&quot;media_type&quot;:2,&quot;video&quot;:&quot;https://d35tn3x5zm6xrc.cloudfront.net/clipHash1/hls/clipHash1.m3u8&quot;}]"></div><a href="/activities/18437723885">"#;
        assert_eq!(activity_ids(html), vec!["18437723885".to_string()]);
        let media = media_from_activity_html(html, "18437723885");
        assert_eq!(media.len(), 2);
        assert_eq!(media[0].kind, "image");
        assert!(media[0].url.contains("2048"));
        assert_eq!(media[0].filename, "18437723885/photo-01.jpg");
        assert_eq!(media[1].kind, "video");
        assert!(media[1].filename.ends_with(".mp4"));
        assert!(media[0]
            .candidates
            .iter()
            .any(|u| u.ends_with("/pic-2048x1152.jpg")));
    }

    #[test]
    fn detects_login_page() {
        assert!(is_login_page(
            "<title data-next-head=\"\">Log In | Strava</title>"
        ));
        assert!(!is_login_page(
            "<title data-next-head=\"\">K k | Strava Athlete Profile</title>"
        ));
    }

    #[test]
    fn prefers_mp4_over_hls_playlist() {
        let hls = "https://d35tn3x5zm6xrc.cloudfront.net/clipHash/hls/clipHash.m3u8";
        let candidates = video_candidates(&[hls.into()]);
        assert_eq!(
            candidates[0],
            "https://d35tn3x5zm6xrc.cloudfront.net/clipHash/clipHash.mp4"
        );
        assert!(candidates.iter().any(|u| u.ends_with(".m3u8")));
    }

    #[test]
    fn picks_largest_api_photo() {
        let json = serde_json::json!([{
            "urls": {
                "100": "https://dgtzuqphqg23d.cloudfront.net/a-100x100.jpg",
                "2000": "https://dgtzuqphqg23d.cloudfront.net/a-2000x2000.jpg"
            }
        }]);
        let photos = photos_from_api(&json, "99");
        assert_eq!(photos.len(), 1);
        assert!(photos[0].url.contains("2048") || photos[0].url.contains("2000"));
        assert!(photos[0]
            .candidates
            .iter()
            .any(|u| u.contains("2000x2000") || u.contains("2048")));
    }

    #[test]
    fn parses_media_thumbnail_list_and_cursor() {
        let html = r#"<div data-react-class="MediaThumbnailList" data-react-props="{&quot;items&quot;:[{&quot;media_type&quot;:1,&quot;large&quot;:&quot;https://dgtzuqphqg23d.cloudfront.net/pic-2048x1365.jpg&quot;,&quot;thumbnail&quot;:&quot;https://dgtzuqphqg23d.cloudfront.net/pic-128x85.jpg&quot;},{&quot;media_type&quot;:2,&quot;video&quot;:&quot;https://d35tn3x5zm6xrc.cloudfront.net/clipHash1/hls/clipHash1.m3u8&quot;}],&quot;nextCursor&quot;:&quot;1,2&quot;,&quot;hasMore&quot;:true}"></div>"#;
        let page = parse_gallery_html(html).unwrap();
        assert!(page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("1,2"));
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].kind, "image");
        assert!(page.items[0].url.contains("2048x1365"));
        assert_eq!(page.items[1].kind, "video");
        assert!(page.items[1].url.ends_with(".mp4"));
    }

    #[test]
    fn parses_snake_case_gallery_json() {
        let json = r#"{"items":[{"media_type":1,"large":"https://dgtzuqphqg23d.cloudfront.net/a-1536x2048.jpg"}],"next_cursor":"9,8","has_more":false}"#;
        let page = parse_gallery_json(json).unwrap();
        assert!(!page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("9,8"));
        assert_eq!(
            page.items[0].url,
            "https://dgtzuqphqg23d.cloudfront.net/a-1536x2048.jpg"
        );
    }

    #[test]
    fn assigns_flat_sequential_names() {
        let mut items = vec![
            Media::with_candidates(
                "image",
                vec!["https://dgtzuqphqg23d.cloudfront.net/a-2048x1365.jpg".into()],
                "g".into(),
                "x".into(),
                None,
            ),
            Media::with_candidates(
                "video",
                vec!["https://d35tn3x5zm6xrc.cloudfront.net/h/h.mp4".into()],
                "g".into(),
                "y".into(),
                None,
            ),
        ];
        assign_flat_names("135539874", &mut items);
        assert_eq!(items[0].filename, "strava-135539874-001.jpg");
        assert_eq!(items[1].filename, "strava-135539874-002.mp4");
    }

    #[test]
    fn json_activity_ids_skip_athlete_objects() {
        let json = serde_json::json!({
            "athlete": {"id": 135539874, "firstname": "K"},
            "models": [{"id": 111, "name": "Ride", "type": "Ride", "sport_type": "Ride"}]
        });
        assert_eq!(
            activity_ids_from_json(&json, "135539874"),
            vec!["111".to_string()]
        );
    }
}
