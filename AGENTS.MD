# AGENTS.MD

Guidance for coding agents working on gh-view.

## Project overview

gh-view is a Rust terminal application for tracking GitHub pull requests. It uses the GitHub CLI (`gh`) as the transport layer and provides a TUI dashboard, PR detail view, unified discussion carousel, review-thread code context, and quick browser/refresh actions.

## Source of truth

- `README.md`: public project overview and developer commands.
- `PLAN.md`: local implementation plan; do not commit.
- `TASKS.md`: local task tracking if created; do not commit.

## Stack

- Language: Rust
- Package manager/build tool: Cargo
- Planned TUI direction: Ratatui-style terminal UI unless changed by maintainers.
- GitHub integration: shell out to `gh` rather than implementing direct API auth.
- Do not manage GitHub tokens in gh-view. Users must install and authenticate `gh` themselves; gh-view should show clear warnings/errors when `gh` is missing or unauthenticated.

## Development commands

Run before every commit and before handing off changes when possible:

```sh
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Run the app locally:

```sh
cargo run
cargo run -- doctor
cargo run -- --mock
```

Use `--mock` for development/demo runs that should not call `gh`. Live runs must rely on the user's installed and authenticated GitHub CLI.

## Current architecture

- `src/main.rs`: CLI parsing, config loading, top-level command dispatch, and selecting live vs mock data sources.
- `src/config.rs`: optional user config loading from `~/.config/gh-view/config.toml`.
- `src/app.rs`: application state, navigation, async dashboard/detail refresh orchestration, background result polling, active detail-pane state, and browser actions. Keep it UI-framework-light and testable.
- `src/app/rows.rs`: projection from dashboard domain data into visible dashboard rows, including repository expansion state.
- `src/model/`: domain models for dashboards, PRs, repository groups, reviewers, checks, comments, discussion items, and code context. Keep GitHub/Ratatui details out of domain models.
- `src/github/`: GitHub data-source trait, live `gh` client, mock client, command execution, error classification, GraphQL/JSON types, and response parsing. Live code must shell out to user-configured `gh`; do not manage tokens.
- `src/github/gh/types.rs`: JSON/GraphQL response DTOs and conversion into domain models. Keep parsing tests here.
- `src/ui/`: Ratatui terminal setup, render loop, input handling, dashboard rendering, detail rendering, theme, and text helpers. Keep network/GitHub calls out of rendering code.

Important data flow:

- Dashboard loads should stay bulk-oriented: fetch authored PRs and review-requested PRs together when possible, then normalize into `Dashboard`/`RepoGroup` models.
- Opening a PR should use a placeholder from dashboard metadata immediately, then load detail and discussion in background workers.
- Review-thread discussion may load source-file context around the commented line; fall back to GraphQL diff hunks if raw source context is unavailable.

## UI direction

- Use a flat full-screen terminal layout, not boxed dashboard panels.
- Prefer a minimal terminal UI style: compact rows, quiet separators, clear typography, and only enough color to show state and focus.
- Use purple as the primary accent for title/cursor, green for approved/passing/reviewers, blue for needs-review, yellow/orange for changes-requested/pending, and red for failing.
- Keep selection as a gutter marker, not a highlighted row background.
- Keep PR rows indented under repository rows.
- Keep right-side age/CI columns aligned and truncate long PR titles with an ellipsis.
- Keep stale PR hints visible but not noisy; warning-colored age markers are enough unless the product direction changes.
- In PR detail, keep active pane focus visible at pane edges/headers while preserving the flat layout.
- Use mock mode (`--mock`) when tuning UI layout.

## Coding guidelines

- Keep modules small and focused.
- Prefer explicit domain models for PRs, repositories, reviewers, checks, and comments.
- Keep GitHub CLI execution isolated behind `src/github/`.
- Keep dashboard refreshes bulk-oriented. Prefer a single GraphQL request for dashboard-wide data, and avoid per-PR `gh` calls in list views.
- Keep long-running `gh` calls off the TUI event loop. Start them from `src/app.rs`, poll completion, and redraw only after input, resize, or background state changes.
- Avoid network calls in unit tests; mock command execution or test parsing separately.
- Prefer mock/demo data behind the data-source trait for UI development instead of hard-coding fixtures in UI rendering.
- Model PR discussion as a unified list of issue comments and review threads. Code context should be optional on each discussion item, because issue comments do not have file/line context while review threads do.
- Keep user-facing terminal output concise and actionable.
- Do not add secrets, tokens, or machine-specific configuration to the repository.

## Git and local files

- Do not commit `PLAN.md` or `TASKS.md`.
- `AGENTS.MD` may be committed if the maintainers decide agent guidance should travel with the repo; until then, keep it reviewed like source documentation.
