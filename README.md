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

Strava may require an authenticated session for activity media. Pass the browser Cookie header with `--cookie`, or use `STRAVA_COOKIE`. An OAuth API token can be supplied with `--token` or `STRAVA_ACCESS_TOKEN`.

```bash
STRAVA_COOKIE='name=value; ...' strava-dl 135539874
STRAVA_ACCESS_TOKEN='...' strava-dl 135539874
```

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT. See [LICENSE](LICENSE).
