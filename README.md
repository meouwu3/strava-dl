# strava-dl

Download accessible images and videos from a Strava athlete profile.

```bash
cargo install --path .
strava-dl 135539874
strava-dl 135539874 --output zip

# short alias
sdl 135539874
```

Files are written to `~/Downloads/strava-<athlete-id>/` by default. Use `--output-dir` to change the destination. `--output zip` creates a ZIP archive instead. Each archive includes `manifest.json`.

Activity photos and videos require a logged-in session. Profile avatars are not downloaded. Photos are requested at the 2048px CloudFront size used by the athlete photo lightbox, not the 128px strip thumbnails. Videos are saved as MP4.

```bash
strava-dl --cookie 'name=value; ...' 135539874
```

`--cookie` / `STRAVA_COOKIE` is the browser Cookie header. `--token` / `STRAVA_ACCESS_TOKEN` is a Strava OAuth access token and is only useful for the authenticated athlete's own activities. `--csrf` / `STRAVA_CSRF_TOKEN` is optional and used for XHR calendar requests.

Cookie values contain `;`. Always quote them in the shell.

Activity media is grouped by activity id:

```text
strava-135539874/
  manifest.json
  18437723885/
    photo-01.jpg
    video-01.mp4
```

Gallery photos without an activity id are written under `photos/`.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT. See [LICENSE](LICENSE).
