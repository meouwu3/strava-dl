use anyhow::Result;
use serde_json::Value;
use std::collections::HashSet;
use time::OffsetDateTime;

use crate::client::Fetcher;
use crate::extract::{
    activity_ids, activity_ids_from_json, is_login_page, media_from_activity_html, media_from_html,
    media_key, parse_json_or_none, photos_from_api,
};
use crate::model::Media;

pub struct DiscoverOpts<'a> {
    pub base_url: &'a str,
    pub athlete_id: &'a str,
    pub authenticated: bool,
    pub year: i32,
    pub month: u32,
    pub max_activities: Option<usize>,
}

pub fn current_year_month() -> (i32, u32) {
    let now = OffsetDateTime::now_utc().date();
    (now.year(), u8::from(now.month()) as u32)
}

pub fn discover(fetcher: &dyn Fetcher, opts: &DiscoverOpts<'_>) -> Result<Vec<Media>> {
    let profile_url = format!("{}/athletes/{}", opts.base_url, opts.athlete_id);
    let profile = fetcher.get_text(&profile_url, false)?;
    if opts.authenticated && is_login_page(&profile) {
        eprintln!("warning: Strava returned a login page; cookie or token may be expired");
    }

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for item in media_from_html(&profile, &profile_url, None) {
        push_item(&mut items, &mut seen, item);
    }

    let mut ids = Vec::new();
    let mut id_seen = HashSet::new();
    extend_ids(&mut ids, &mut id_seen, activity_ids(&profile));

    if opts.authenticated {
        collect_from_photos_pages(fetcher, opts, &mut ids, &mut id_seen, &mut items, &mut seen)?;
        collect_from_interval(fetcher, opts, &mut ids, &mut id_seen)?;
        collect_from_api(fetcher, opts, &mut ids, &mut id_seen, &mut items, &mut seen)?;
    }

    if let Some(max) = opts.max_activities {
        ids.truncate(max);
    }

    for (index, activity_id) in ids.iter().enumerate() {
        eprint!("\rscanning activities {}/{}", index + 1, ids.len());
        let url = format!("{}/activities/{activity_id}", opts.base_url);
        match fetcher.get_text(&url, false) {
            Ok(html) => {
                if is_login_page(&html) {
                    continue;
                }
                for item in media_from_activity_html(&html, activity_id) {
                    push_item(&mut items, &mut seen, item);
                }
            }
            Err(_) => continue,
        }
    }
    if !ids.is_empty() {
        eprintln!();
    }

    Ok(items)
}

fn collect_from_photos_pages(
    fetcher: &dyn Fetcher,
    opts: &DiscoverOpts<'_>,
    ids: &mut Vec<String>,
    id_seen: &mut HashSet<String>,
    items: &mut Vec<Media>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    for page in 1..=50 {
        let url = if page == 1 {
            format!("{}/athletes/{}/photos", opts.base_url, opts.athlete_id)
        } else {
            format!(
                "{}/athletes/{}/photos?page={page}",
                opts.base_url, opts.athlete_id
            )
        };
        let Ok(body) = fetcher.get_text(&url, page > 1) else {
            break;
        };
        if is_login_page(&body) {
            break;
        }
        let before_ids = ids.len();
        let before_media = items.len();
        extend_ids(ids, id_seen, activity_ids(&body));
        if let Some(json) = parse_json_or_none(&body) {
            extend_ids(ids, id_seen, activity_ids_from_json(&json, opts.athlete_id));
        }
        for item in media_from_html(&body, &url, None) {
            push_item(items, seen, item);
        }
        if page > 1 && ids.len() == before_ids && items.len() == before_media {
            break;
        }
    }
    Ok(())
}

fn collect_from_interval(
    fetcher: &dyn Fetcher,
    opts: &DiscoverOpts<'_>,
    ids: &mut Vec<String>,
    id_seen: &mut HashSet<String>,
) -> Result<()> {
    let mut year = opts.year;
    let mut month = opts.month;
    let mut empty = 0;
    for _ in 0..(15 * 12) {
        let interval = format!("{year}{month:02}");
        let url = format!(
            "{}/athletes/{}/interval?interval={interval}&interval_type=month&chart_type=miles&year_offset=0",
            opts.base_url, opts.athlete_id
        );
        match fetcher.get_text(&url, true) {
            Ok(body) => {
                if is_login_page(&body) {
                    break;
                }
                let before = ids.len();
                extend_ids(ids, id_seen, activity_ids(&body));
                if let Some(json) = parse_json_or_none(&body) {
                    extend_ids(ids, id_seen, activity_ids_from_json(&json, opts.athlete_id));
                }
                if ids.len() == before {
                    empty += 1;
                } else {
                    empty = 0;
                }
            }
            Err(_) => {
                empty += 1;
            }
        }
        if empty >= 6 {
            break;
        }
        if month == 1 {
            month = 12;
            year -= 1;
        } else {
            month -= 1;
        }
        if year < opts.year - 15 {
            break;
        }
    }
    Ok(())
}

fn collect_from_api(
    fetcher: &dyn Fetcher,
    opts: &DiscoverOpts<'_>,
    ids: &mut Vec<String>,
    id_seen: &mut HashSet<String>,
    items: &mut Vec<Media>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    let me = fetcher.get_text(&format!("{}/api/v3/athlete", opts.base_url), false);
    let Ok(body) = me else {
        return Ok(());
    };
    let Some(json) = parse_json_or_none(&body) else {
        return Ok(());
    };
    let Some(me_id) = json.get("id").map(|v| v.to_string().replace('"', "")) else {
        return Ok(());
    };
    if me_id != opts.athlete_id {
        return Ok(());
    }
    for page in 1..=200 {
        let url = format!(
            "{}/api/v3/athlete/activities?per_page=200&page={page}",
            opts.base_url
        );
        let Ok(body) = fetcher.get_text(&url, false) else {
            break;
        };
        let Some(Value::Array(list)) = parse_json_or_none(&body) else {
            break;
        };
        if list.is_empty() {
            break;
        }
        for activity in &list {
            let Some(id) = activity.get("id").map(|v| v.to_string().replace('"', "")) else {
                continue;
            };
            extend_ids(ids, id_seen, vec![id.clone()]);
            let photo_count = activity
                .get("total_photo_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if photo_count == 0 {
                continue;
            }
            let photos_url = format!(
                "{}/api/v3/activities/{id}/photos?photo_sources=true&size=5000",
                opts.base_url
            );
            if let Ok(photos_body) = fetcher.get_text(&photos_url, false) {
                if let Some(photos) = parse_json_or_none(&photos_body) {
                    for item in photos_from_api(&photos, &id) {
                        push_item(items, seen, item);
                    }
                }
            }
        }
        if list.len() < 200 {
            break;
        }
    }
    Ok(())
}

fn extend_ids(ids: &mut Vec<String>, seen: &mut HashSet<String>, more: Vec<String>) {
    for id in more {
        if seen.insert(id.clone()) {
            ids.push(id);
        }
    }
}

fn push_item(items: &mut Vec<Media>, seen: &mut HashSet<String>, item: Media) {
    if seen.insert(media_key(&item.url)) {
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Fetcher;
    use anyhow::{bail, Result};
    use std::collections::HashMap;

    struct MapFetcher {
        text: HashMap<String, String>,
    }

    impl Fetcher for MapFetcher {
        fn get_text(&self, url: &str, _xhr: bool) -> Result<String> {
            self.text
                .get(url)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing {url}"))
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
            bail!("no bytes {url}")
        }
    }

    #[test]
    fn public_profile_skips_avatar() {
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"athlete":{"profileImageUrl":"https://dgalywyr863hv.cloudfront.net/pictures/athletes/135539874/30856735/12/medium.jpg"}}}}</script>"#;
        let fetcher = MapFetcher {
            text: HashMap::from([(
                "https://www.strava.com/athletes/135539874".into(),
                html.into(),
            )]),
        };
        let opts = DiscoverOpts {
            base_url: "https://www.strava.com",
            athlete_id: "135539874",
            authenticated: false,
            year: 2026,
            month: 9,
            max_activities: None,
        };
        let items = discover(&fetcher, &opts).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn authenticated_scan_reads_activity_media() {
        let profile = r#"<title>K k | Strava Athlete Profile</title><a href="/activities/99">"#;
        let activity = r#"[{ "media_type": 1, "large": "https://dgtzuqphqg23d.cloudfront.net/pic-2048x1152.jpg", "video": null }]"#;
        let fetcher = MapFetcher {
            text: HashMap::from([
                ("https://www.strava.com/athletes/1".into(), profile.into()),
                (
                    "https://www.strava.com/activities/99".into(),
                    activity.into(),
                ),
            ]),
        };
        let opts = DiscoverOpts {
            base_url: "https://www.strava.com",
            athlete_id: "1",
            authenticated: true,
            year: 2026,
            month: 9,
            max_activities: Some(1),
        };
        let items = discover(&fetcher, &opts).unwrap();
        assert!(items.iter().any(|m| m.filename == "99/photo-01.jpg"));
    }
}
