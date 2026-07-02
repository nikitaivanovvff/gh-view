# gh-view

A terminal view for GitHub pull requests.

gh-view helps you track PRs opened by you and PRs awaiting your review across repositories, with reviewer state, CI status, PR details, comments, and quick actions powered by the GitHub CLI.

gh-view is an independent project and is not affiliated with, endorsed by, or sponsored by GitHub, Inc.

## Requirements

- Rust toolchain
- GitHub CLI (`gh`) for live GitHub integration
  - gh-view does not manage GitHub tokens; authenticate with `gh auth login` yourself.
  - Use `--mock` to run the dashboard with built-in development data and no `gh` calls.

## Development

```sh
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- doctor
```

## Usage

```sh
cargo run
cargo run -- dashboard
cargo run -- doctor
cargo run -- --mock
cargo run -- --mock doctor
```
