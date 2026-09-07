use anyhow::Result;
use std::collections::HashSet;

use crate::client::Fetcher;
use crate::extract::{
    is_login_page, media_key, parse_gallery_html, parse_gallery_json, GalleryPage,
};
use crate::model::Media;

pub struct DiscoverOpts<'a> {
    pub base_url: &'a str,
    pub athlete_id: &'a str,
    pub max_items: Option<usize>,
}

pub fn discover(fetcher: &dyn Fetcher, opts: &DiscoverOpts<'_>) -> Result<Vec<Media>> {
    let profile_url = format!("{}/athletes/{}", opts.base_url, opts.athlete_id);
    let profile = fetcher.get_text(&profile_url, false)?;
    if is_login_page(&profile) {
        eprintln!("warning: Strava returned a login page; cookie or token may be expired");
    }

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut cursor = None;
    let mut has_more = false;

    if let Some(page) = parse_gallery_html(&profile) {
        merge_page(&mut items, &mut seen, page, &mut cursor, &mut has_more);
    }
    if items.is_empty() {
        let url = format!(
            "{}/athletes/{}/photos?per_page=20",
            opts.base_url, opts.athlete_id
        );
        if let Ok(body) = fetcher.get_text(&url, true) {
            if let Some(page) = parse_gallery_json(&body) {
                merge_page(&mut items, &mut seen, page, &mut cursor, &mut has_more);
            }
        }
    }

    let mut pages = 1;
    while has_more {
        if let Some(max) = opts.max_items {
            if items.len() >= max {
                break;
            }
        }
        let Some(current) = cursor.clone() else {
            break;
        };
        pages += 1;
        if pages > 200 {
            break;
        }
        eprint!("\rscanning photos {pages}");
        let url = format!(
            "{}/athletes/{}/photos?cursor={}&per_page=20",
            opts.base_url,
            opts.athlete_id,
            current.replace(',', "%2C")
        );
        match fetcher.get_text(&url, true) {
            Ok(body) => {
                let Some(page) = parse_gallery_json(&body) else {
                    break;
                };
                let before = items.len();
                merge_page(&mut items, &mut seen, page, &mut cursor, &mut has_more);
                if items.len() == before {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if pages > 1 {
        eprintln!();
    }
    if let Some(max) = opts.max_items {
        items.truncate(max);
    }
    Ok(items)
}

fn merge_page(
    items: &mut Vec<Media>,
    seen: &mut HashSet<String>,
    page: GalleryPage,
    cursor: &mut Option<String>,
    has_more: &mut bool,
) {
    for item in page.items {
        if seen.insert(media_key(&item.url)) {
            items.push(item);
        }
    }
    *cursor = page.next_cursor;
    *has_more = page.has_more;
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
    fn public_profile_without_gallery_is_empty() {
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
            max_items: None,
        };
        let items = discover(&fetcher, &opts).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn paginates_athlete_photo_gallery() {
        let profile = r#"<div data-react-class="MediaThumbnailList" data-react-props="{&quot;items&quot;:[{&quot;media_type&quot;:1,&quot;large&quot;:&quot;https://dgtzuqphqg23d.cloudfront.net/aaa-2048x1365.jpg&quot;}],&quot;nextCursor&quot;:&quot;1,2&quot;,&quot;hasMore&quot;:true}"></div>"#;
        let page2 = r#"{"items":[{"media_type":2,"video":"https://d35tn3x5zm6xrc.cloudfront.net/clipHash1/hls/clipHash1.m3u8"}],"next_cursor":null,"has_more":false}"#;
        let fetcher = MapFetcher {
            text: HashMap::from([
                ("https://www.strava.com/athletes/1".into(), profile.into()),
                (
                    "https://www.strava.com/athletes/1/photos?cursor=1%2C2&per_page=20".into(),
                    page2.into(),
                ),
            ]),
        };
        let opts = DiscoverOpts {
            base_url: "https://www.strava.com",
            athlete_id: "1",
            max_items: None,
        };
        let items = discover(&fetcher, &opts).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, "image");
        assert_eq!(items[1].kind, "video");
    }
}
