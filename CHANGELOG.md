# Changelog

## Unreleased

## 0.3.5

- Show a scan progress bar with the current page, and a download progress bar with current/total.
- Print `下载完成` on success and `下载失败` on error.

## 0.3.4

- Number files oldest first. Strava lists newest media first; `strava-{id}-001` is the earliest item.

## 0.3.3

- Discover media from the athlete Photos lightbox API (`/athletes/{id}/photos?cursor=`) instead of the incomplete activity list.
- Write a flat archive named `strava-{id}-001.jpg` / `strava-{id}-002.mp4` with no activity subfolders.

## 0.3.2

- Download lightbox-sized photos by requesting 2048px CloudFront variants instead of unsized thumbnail URLs.
- Save videos as MP4 from the CDN (`/{hash}/{hash}.mp4`) instead of concatenated `.m3u8`.
- Collect media from the athlete profile photo strip as well as activity pages.

## 0.3.1

- Skip athlete profile avatars.
- Prefer original photo files, then the largest sized CDN variant.
- Prefer MP4 video files when present; otherwise download the highest-resolution HLS variant.

## 0.3.0

- Discover activity photos and videos from athlete pages, photo tabs, monthly calendars, and the official API when the token belongs to that athlete.
- Download original-resolution photos when the CDN exposes them.
- Download HLS videos as concatenated `.ts` files.
- Split the crate into focused modules with parser, archive, and discovery tests.
- Add optional `--csrf` for calendar XHR requests.

## 0.2.0

- Native Rust CLI with `strava-dl` and `sdl` commands.
- Folder and ZIP archive output.
- Cookie and OAuth token authentication options.
