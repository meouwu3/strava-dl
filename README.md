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

Without authentication, only publicly visible profile media is available. Activity photos and videos require a logged-in session:

```bash
strava-dl --cookie 'name=value; ...' 135539874
```

`--cookie` / `STRAVA_COOKIE` is the browser Cookie header. `--token` / `STRAVA_ACCESS_TOKEN` is a Strava OAuth access token and is only useful for the authenticated athlete's own activities. `--csrf` / `STRAVA_CSRF_TOKEN` is optional and used for XHR calendar requests.

Cookie values contain `;`. Always quote them in the shell.

Activity media is grouped by activity id:

```text
strava-135539874/
  manifest.json
  profile.jpg
  18437723885/
    photo-01.jpg
    video-01.ts
```

HLS videos are saved as concatenated `.ts` files. Remux with `ffmpeg -i video-01.ts -c copy video-01.mp4` if you need MP4.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT. See [LICENSE](LICENSE).
