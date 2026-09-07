# Contributing

Before submitting a change:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Keep changes focused, add regression coverage for behavior changes, and update the README when command-line behavior changes. Do not commit credentials, session cookies, downloaded media, or `target/`.
