use crate::model::Media;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const STATIC_HOSTS: &[&str] = &["d3nn82uaxijpm6.cloudfront.net", "web-assets.strava.com"];

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
        if name == "medium.jpg" {
            format!("{prefix}large.jpg")
        } else {
            let stripped = regex_lite_strip_size(name);
            format!("{prefix}{stripped}")
        }
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

pub fn profile_image_url(html: &str) -> Option<String> {
    if let Some(url) = next_data_profile_image(html) {
        return Some(original_photo_url(&url));
    }
    extract_urls(html)
        .into_iter()
        .find(|url| url.contains("/pictures/athletes/"))
        .map(|url| original_photo_url(&url))
}

fn next_data_profile_image(html: &str) -> Option<String> {
    let start = html.find("id=\"__NEXT_DATA__\"")?;
    let after = &html[start..];
    let gt = after.find('>')?;
    let rest = &after[gt + 1..];
    let end = rest.find("</script>")?;
    let json: Value = serde_json::from_str(&rest[..end]).ok()?;
    json.pointer("/props/pageProps/athlete/profileImageUrl")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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

pub fn filename_from_url(url: &str, fallback: &str) -> String {
    let clean = original_photo_url(url);
    let path = clean.split('?').next().unwrap_or(&clean);
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub fn media_from_activity_html(html: &str, activity_id: &str) -> Vec<Media> {
    let source = format!("https://www.strava.com/activities/{activity_id}");
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut photo_i = 0u32;
    let mut video_i = 0u32;

    for raw in parse_media_type_entries(html) {
        let (kind, url) = match media_from_entry(&raw) {
            Some(v) => v,
            None => continue,
        };
        if !seen.insert(url.clone()) {
            continue;
        }
        items.push(named_media(
            kind,
            url,
            source.clone(),
            activity_id,
            &mut photo_i,
            &mut video_i,
        ));
    }

    for url in extract_urls(html) {
        let Some(kind) = media_kind(&url) else {
            continue;
        };
        let url = if kind == "image" {
            original_photo_url(&url)
        } else {
            unescape_url(&url)
        };
        if !seen.insert(url.clone()) {
            continue;
        }
        items.push(named_media(
            kind,
            url,
            source.clone(),
            activity_id,
            &mut photo_i,
            &mut video_i,
        ));
    }
    items
}

fn named_media(
    kind: &str,
    url: String,
    source: String,
    activity_id: &str,
    photo_i: &mut u32,
    video_i: &mut u32,
) -> Media {
    if kind == "video" {
        *video_i += 1;
        let ext = extension(&url).unwrap_or("ts");
        let filename = format!("{activity_id}/video-{:02}.{ext}", *video_i);
        Media::video(url, source, filename, Some(activity_id.into()))
    } else {
        *photo_i += 1;
        let ext = extension(&url).unwrap_or("jpg");
        let filename = format!("{activity_id}/photo-{:02}.{ext}", *photo_i);
        Media::image(url, source, filename, Some(activity_id.into()))
    }
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

fn media_from_entry(value: &Value) -> Option<(&str, String)> {
    let media_type = value.get("media_type").and_then(Value::as_u64).unwrap_or(1);
    if media_type == 2 {
        let url = value
            .get("video")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())?;
        return Some(("video", unescape_url(url)));
    }
    let url = value
        .get("large")
        .and_then(Value::as_str)
        .or_else(|| value.get("thumbnail").and_then(Value::as_str))
        .filter(|u| !u.is_empty())?;
    Some(("image", original_photo_url(url)))
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
            if !url.is_empty() && seen.insert(url.to_string()) {
                items.push(named_media(
                    "video",
                    unescape_url(url),
                    source.clone(),
                    activity_id,
                    &mut photo_i,
                    &mut video_i,
                ));
            }
        }
        if let Some(url) = best_sized_url(entry.get("urls")) {
            if seen.insert(url.clone()) {
                items.push(named_media(
                    "image",
                    original_photo_url(&url),
                    source.clone(),
                    activity_id,
                    &mut photo_i,
                    &mut video_i,
                ));
            }
        }
    }
    items
}

fn best_sized_url(urls: Option<&Value>) -> Option<String> {
    match urls? {
        Value::String(s) => Some(s.clone()),
        Value::Object(map) => {
            let mut ranked: BTreeMap<i64, String> = BTreeMap::new();
            for (key, value) in map {
                if let Some(url) = value.as_str() {
                    let score = key.parse::<i64>().unwrap_or(0);
                    ranked.insert(score, url.to_string());
                }
            }
            ranked.last_key_value().map(|(_, url)| url.clone())
        }
        _ => None,
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
    fn strips_size_suffix_and_upgrades_profile_medium() {
        assert_eq!(
            original_photo_url("https://dgtzuqphqg23d.cloudfront.net/hash-2048x1152.jpg"),
            "https://dgtzuqphqg23d.cloudfront.net/hash.jpg"
        );
        assert_eq!(
            original_photo_url(
                "https://dgalywyr863hv.cloudfront.net/pictures/athletes/1/2/12/medium.jpg"
            ),
            "https://dgalywyr863hv.cloudfront.net/pictures/athletes/1/2/12/large.jpg"
        );
    }

    #[test]
    fn ignores_static_assets() {
        assert!(!is_strava_media_url(
            "https://d3nn82uaxijpm6.cloudfront.net/favicon-32x32.png"
        ));
        assert!(is_strava_media_url(
            "https://dgtzuqphqg23d.cloudfront.net/hash-2048x1152.jpg"
        ));
        assert!(is_strava_media_url(
            "https://dgalywyr863hv.cloudfront.net/pictures/athletes/1/2/12/medium.jpg"
        ));
    }

    #[test]
    fn extracts_activity_ids_and_media_blob() {
        let html = r#"<div data-react-props="[{&quot;media_type&quot;:1,&quot;large&quot;:&quot;https://dgtzuqphqg23d.cloudfront.net/pic-2048x1152.jpg&quot;,&quot;video&quot;:null},{&quot;media_type&quot;:2,&quot;video&quot;:&quot;https://d35tn3x5zm6xrc.cloudfront.net/clip.m3u8&quot;}]"></div><a href="/activities/18437723885">"#;
        assert_eq!(activity_ids(html), vec!["18437723885".to_string()]);
        let media = media_from_activity_html(html, "18437723885");
        assert_eq!(media.len(), 2);
        assert_eq!(media[0].kind, "image");
        assert_eq!(media[0].url, "https://dgtzuqphqg23d.cloudfront.net/pic.jpg");
        assert_eq!(media[0].filename, "18437723885/photo-01.jpg");
        assert_eq!(media[1].kind, "video");
        assert_eq!(media[1].filename, "18437723885/video-01.m3u8");
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
    fn reads_profile_image_from_next_data() {
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"athlete":{"profileImageUrl":"https://dgalywyr863hv.cloudfront.net/pictures/athletes/135539874/30856735/12/medium.jpg"}}}}</script>"#;
        assert_eq!(
            profile_image_url(html).as_deref(),
            Some(
                "https://dgalywyr863hv.cloudfront.net/pictures/athletes/135539874/30856735/12/large.jpg"
            )
        );
    }

    #[test]
    fn picks_largest_api_photo() {
        let json = serde_json::json!([{
            "urls": {"100": "https://cdn.example/a-100.jpg", "2000": "https://dgtzuqphqg23d.cloudfront.net/a-2000x2000.jpg"}
        }]);
        let photos = photos_from_api(&json, "99");
        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].url, "https://dgtzuqphqg23d.cloudfront.net/a.jpg");
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
