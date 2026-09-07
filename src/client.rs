use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, REFERER, USER_AGENT};
use std::time::Duration;

pub trait Fetcher {
    fn get_text(&self, url: &str, xhr: bool) -> Result<String>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct StravaClient {
    inner: Client,
    cookie: Option<String>,
    token: Option<String>,
    csrf: Option<String>,
}

impl StravaClient {
    pub fn new(
        timeout_secs: u64,
        cookie: Option<String>,
        token: Option<String>,
        csrf: Option<String>,
    ) -> Result<Self> {
        let inner = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            inner,
            cookie,
            token,
            csrf,
        })
    }

    fn headers(&self, xhr: bool) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(REFERER, HeaderValue::from_static("https://www.strava.com/"));
        if let Some(cookie) = &self.cookie {
            headers.insert(
                COOKIE,
                HeaderValue::from_str(cookie).context("cookie header is not valid HTTP")?,
            );
        }
        if let Some(token) = &self.token {
            let value = format!("Bearer {token}");
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value).context("token header is not valid HTTP")?,
            );
        }
        if xhr {
            headers.insert(
                "x-requested-with",
                HeaderValue::from_static("XMLHttpRequest"),
            );
            headers.insert(
                reqwest::header::ACCEPT,
                HeaderValue::from_static("application/json, text/javascript, */*; q=0.01"),
            );
            if let Some(csrf) = &self.csrf {
                headers.insert(
                    "x-csrf-token",
                    HeaderValue::from_str(csrf).context("csrf header is not valid HTTP")?,
                );
            }
        }
        Ok(headers)
    }
}

impl Fetcher for StravaClient {
    fn get_text(&self, url: &str, xhr: bool) -> Result<String> {
        let response = self
            .inner
            .get(url)
            .headers(self.headers(xhr)?)
            .send()
            .with_context(|| format!("GET {url}"))?
            .error_for_status()
            .with_context(|| format!("GET {url}"))?;
        response.text().with_context(|| format!("read {url}"))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .inner
            .get(url)
            .headers(self.headers(false)?)
            .send()
            .with_context(|| format!("GET {url}"))?
            .error_for_status()
            .with_context(|| format!("GET {url}"))?;
        let bytes = response.bytes().with_context(|| format!("read {url}"))?;
        Ok(bytes.to_vec())
    }
}

pub fn looks_like_media(bytes: &[u8]) -> bool {
    if bytes.len() < 16 {
        return false;
    }
    !bytes.starts_with(b"<") && !bytes.starts_with(b"{") && !bytes.starts_with(b"[")
}
