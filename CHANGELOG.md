# Changelog

## Unreleased

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
