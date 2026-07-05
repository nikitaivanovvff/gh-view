# gh-view

A fast terminal view for GitHub pull requests.

`gh-view` helps you keep track of the PRs you need to care about: pull requests opened by you, pull requests waiting for your review, CI/review state, PR descriptions, comments, review threads, and code context — all from the terminal.

It uses the official GitHub CLI (`gh`) as the transport layer. `gh-view` does not manage GitHub tokens or authentication.

> `gh-view` is an independent project and is not affiliated with, endorsed by, or sponsored by GitHub, Inc.

## Features

- Dashboard grouped by repository
- Sections for:
  - PRs opened by you
  - PRs awaiting your review
- Compact PR rows with review state, CI state, reviewers, and age
- PR detail view with description, branch/state/mergeability metadata, and discussion
- Unified discussion carousel for issue comments and review threads
- Review-thread code context rendered next to comments
- Background loading for PR details and review-thread context
- Mock mode for demos and local UI development without calling GitHub

## Requirements

- GitHub CLI (`gh`)
- An authenticated `gh` session:

```sh
gh auth login
```

Check your local setup with:

```sh
gh-view doctor
```

## Installation

### Homebrew

Homebrew packaging is coming soon:

```sh
brew tap nikitaivanovvff/tap
brew install gh-view
```

## Usage

Launch the dashboard:

```sh
gh-view
```

Explicit dashboard command:

```sh
gh-view dashboard
```

Check dependencies/authentication:

```sh
gh-view doctor
```

## Configuration

Optional configuration is read from `~/.config/gh-view/config.toml`.

```toml
# How long gh-view waits for a `gh` command before stopping it.
# Default: 30 seconds.
gh_timeout_seconds = 30
```

## Keybindings

### Dashboard

| Key | Action |
| --- | --- |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `enter` | Open selected PR |
| `space` / `o` | Collapse/expand repository |
| `b` | Open selected PR in browser |
| `r` | Refresh dashboard |
| `q` / `esc` | Quit |

### PR detail

| Key | Action |
| --- | --- |
| `j` / `↓` | Scroll active pane down |
| `k` / `↑` | Scroll active pane up |
| `tab` | Switch active pane |
| `d` | Focus description pane |
| `D` | Focus discussion pane |
| `n` / `→` | Next discussion item |
| `p` / `←` | Previous discussion item |
| `b` | Open PR in browser |
| `r` | Refresh PR detail |
| `q` / `esc` | Back to dashboard |

## Mock demo data

Use mock mode to try the UI without a GitHub account or network calls:

```sh
gh-view --mock
```

The mock data includes several repositories, PR review states, CI states, review-thread comments, replies, and code context.

## Development

```sh
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- --mock
```

Release builds are produced by GitHub Actions when a version tag is pushed:

```sh
git tag v0.0.1
git push origin v0.0.1
```

## License

MIT
