# AGENTS.MD

Guidance for coding agents working on gh-view.

## Project overview

gh-view is a Rust terminal application for tracking GitHub pull requests. It uses the GitHub CLI (`gh`) as the transport layer and provides a TUI dashboard, PR detail view, unified discussion carousel, review-thread code context, and quick browser/refresh actions.

## Source of truth

- `README.md`: public project overview and developer commands.
- `docs/specs/`: local implementation plans and handoff notes; these may describe planned work rather than current behavior.
- The source code and tests define current behavior when a spec is stale.

## Stack

- Language: Rust 2024 edition.
- Package manager/build tool: Cargo
- TUI: Ratatui with Crossterm terminal/event handling.
- Supporting libraries: `nucleo-matcher` for dashboard fuzzy search and `toml_edit` for structure-preserving config updates.
- GitHub integration: shell out to `gh` rather than implementing direct API auth.
- Do not manage GitHub tokens. Resolve `gh` from `PATH`, require users to authenticate it themselves, enforce the configured command timeout, and surface missing/auth/timeout errors clearly.

## Development commands

Run before every commit and before handing off changes when possible. Use `cargo fmt --check` when verifying without changing files; run `cargo fmt` when formatting is needed.

```sh
cargo fmt
cargo fmt --check
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

- `src/main.rs`: CLI parsing, config loading, command dispatch, live/mock source selection, and UI startup.
- `src/config.rs`: typed config defaults/loading and theme persistence. Config path precedence is `GH_VIEW_CONFIG`, then `$XDG_CONFIG_HOME/gh-view/config.toml`, then `$HOME/.config/gh-view/config.toml`. Theme saves use `toml_edit` and must preserve unrelated keys, tables, comments, and formatting.
- `src/app.rs`: top-level application state, navigation, async worker startup/polling, refresh throttling, search/detail/browser actions, theme-picker state, and transient feedback. Keep it UI-framework-light and testable.
- `src/app/dashboard.rs`: dashboard state, selection, repository collapse state, dashboard search state mutation, and visible-row/search-result coordination.
- `src/app/detail.rs`: detail/discussion state, pane focus and scrolling, result application, and discussion ordering.
- `src/app/rows.rs`: projection from dashboard domain data into visible dashboard rows, including repository expansion state.
- `src/app/search.rs`: dashboard fuzzy-finder candidate construction and `nucleo-matcher` scoring. Build candidates from loaded dashboard data, not visible rows, so collapsed groups remain searchable.
- `src/app/status.rs`: app status classification and shared PR status derivation used by UI and search.
- `src/model/`: domain models for dashboards, PRs, repository groups, reviewers, checks, discussion items, replies, and optional review-thread code context. Keep GitHub DTOs and Ratatui types out of domain models.
- `src/github/source.rs`: live/mock data-source boundary and bulk dashboard contract.
- `src/github/command.rs`: injectable command-runner boundary, PATH-resolved `gh` process execution, concurrent stdout/stderr draining, timeout/kill behavior, and spawn-error classification.
- `src/github/client.rs` and `src/github/queries.rs`: live command composition, GraphQL pagination/deduplication, detail fetching, and review-thread pagination. Dashboard GraphQL falls back to `gh search prs` only for classified generic command failures, never for missing `gh`, authentication, outage, timeout, or parse failures.
- `src/github/mock.rs` and `src/github/error.rs`: deterministic demo/error source and transport error classification.
- `src/github/gh/types.rs`: JSON/GraphQL response DTOs and conversion into domain models. Keep parsing tests here.
- `src/ui/terminal.rs`: Crossterm terminal lifecycle, event loop, and restoration; `render.rs`: top-level render dispatch; `input.rs`: keyboard/mouse dispatch; `layout.rs`: shared geometry.
- `src/ui/dashboard.rs` and `src/ui/detail.rs`: view rendering; `footer.rs`: contextual controls/status; `text.rs`: terminal text helpers.
- `src/ui/theme.rs`: semantic style roles and active-theme selection; `palette.rs`: dark and light palette values; `theme_picker.rs`: picker popup layout, rendering, and hit testing.

Important data flow:

- Dashboard loads should stay bulk-oriented: fetch authored PRs and review-requested PRs together when possible, then normalize into `Dashboard`/`RepoGroup` models.
- Opening a PR uses dashboard metadata as an immediate placeholder, then starts independent detail and discussion workers. Either may complete first: detail results replace detail fields and issue comments while preserving current review threads; discussion results replace review threads, not append stale copies.
- Model discussion as one chronologically sorted list of issue comments and review threads. Review threads may contain replies and optional GraphQL diff-hunk code context; issue comments have no code context.
- Keep long-running `gh` calls off the event loop. Poll worker channels and redraw only after input, resize, animation ticks, or background state changes.

## UI direction

- Use a flat full-screen terminal layout, not boxed dashboard panels.
- Prefer a minimal terminal UI style: compact rows, quiet separators, clear typography, and only enough color to show state and focus.
- Style by semantic theme roles (`accent`, `success`, `info`, `warning`, `danger`, `reviewer`, `branch`, `muted`, rules, selection), not literal colors. Every palette must define an intentional background and readable contrast for both dark and light themes.
- Dashboard selection is the gutter-marker exception: do not add a row background there. Popup/list selection, including search and theme picker, may use the semantic selection background.
- Keep PR rows indented under repository rows.
- Keep right-side age/CI columns aligned and truncate long PR titles with an ellipsis.
- Keep stale PR hints visible but not noisy; warning-colored age markers are enough unless the product direction changes.
- In PR detail, keep active pane focus visible at pane edges/headers while preserving the flat layout.
- Theme-picker movement previews immediately; save persists the selected theme and closes; cancel restores the theme active when the picker opened and does not write config.
- Use mock mode (`--mock`) when tuning UI layout.

## Coding guidelines

- Keep modules small and focused.
- Prefer small, logically independent Rust files with one clear responsibility. Split parsing, rendering helpers, state transitions, and transport concerns when a file starts mixing responsibilities.
- Prefer explicit domain models for PRs, repositories, reviewers, checks, and discussion items.
- Keep GitHub CLI execution isolated behind `src/github/`.
- Keep dashboard refreshes bulk-oriented. Prefer a single GraphQL request for dashboard-wide data, and avoid per-PR `gh` calls in list views.
- Avoid network calls in unit tests; mock command execution or test parsing separately.
- Prefer mock/demo data behind the data-source trait for UI development instead of hard-coding fixtures in UI rendering.
- Keep rendering immutable: render functions should derive widgets from `&App` and must not mutate application state, perform I/O, or call GitHub.
- Keep user-facing terminal output concise and actionable.
- Do not add secrets, tokens, or machine-specific configuration to the repository.

## Git and local files

- Prefer small, independently reviewable pull requests, with one branch and PR per logical task. Split parallel work only when the changes are independent and unlikely to touch the same behavior or files; when tasks overlap or one depends on another, combine them into one coherent branch or implement them as sequential branches based on the preceding change to avoid conflicts and duplicated work.
- Feature specs used to hand off work between coding sessions should live under `docs/specs/` and stay untracked unless maintainers explicitly decide to commit them.
- `AGENTS.MD` is explicitly adopted as repository guidance if committed; until then, keep this ignored/untracked copy reviewed like source documentation.
