# gh-view Design System

This document defines the product language and interaction rules for gh-view's terminal UI. It describes the current design, records known inconsistencies, and guides incremental improvements. Source code and tests remain the behavioral source of truth.

The system is intentionally small. It is a set of shared decisions, not a component framework. New abstractions should be introduced only when repeated behavior cannot stay consistent otherwise.

## Product Principles

1. **Terminal native.** Use a flat, full-screen layout, quiet rules, whitespace, and indentation. Boxes are reserved for modal overlays, not ordinary page structure.
2. **Simple before dense.** Show the information needed to choose the next action. Remove secondary metadata before compressing essential content into noise.
3. **Predictable hierarchy.** The same kind of information appears in the same order, notation, and style across views.
4. **Readable without color.** Text, position, symbols, and modifiers communicate meaning. Color reinforces meaning but never supplies the only cue for important state.
5. **Stable under pressure.** Long GitHub data and small terminals degrade deliberately. User-controlled text must not displace essential navigation or status.
6. **Contextual interaction.** Show controls only when they can act on the current view or selection.
7. **Small implementation surface.** Prefer formatting helpers and explicit width budgets over a general widget or token framework.

## Visual Principles

- Keep the background visually continuous across each page.
- Use horizontal and vertical rules to separate regions without creating nested panels.
- Keep rows compact and repository groups easy to scan.
- Use accent sparingly for identity, primary navigation, selection, and focus.
- Avoid decorative icons when a short textual label is clearer.
- Preserve terminal conventions: uppercase section labels, lowercase controls, monospace alignment, and mnemonic keys.

## Information Hierarchy

From strongest to quietest:

1. Application identity, selected item, active primary view, and focused pane.
2. PR title and number, repository, actionable status, and blocking errors.
3. Review/CI state, author or reviewer identity, branch, age, and navigation position.
4. Help text, inactive controls, descriptions, and decorative separators.

Primary navigation is uppercase and accent-emphasized: `MY PRS [2]`, `AWAITING REVIEW [7]`, `DESCRIPTION`, and `DISCUSSION`. Secondary controls are lowercase and muted: `all [7]`, `direct [6]`, `team [2]`, and footer labels. An active secondary control may add restrained bold and underline; it must not compete with primary navigation.

## Spacing And Indentation

- Use one blank column between a gutter marker and row content.
- Repository rows begin at the page hierarchy level.
- PR rows are indented four columns beneath a repository row.
- PR continuation lines align beneath the PR's primary content, not beneath the selection gutter.
- Use two spaces between related metadata fields.
- Use three spaces between distinct footer controls and between right-side metadata groups.
- Use one quiet rule between major regions. Do not add blank rows merely to simulate cards.
- Overlay borders consume one cell on every side; calculate content from the resulting inner rectangle.
- Mouse regions use the exact rendered row and column geometry. They must not be inferred independently from domain indexes or fixed screen coordinates.

Spacing is a hierarchy tool, not decoration. At narrow widths, remove an entire lower-priority field before reducing meaningful separation around the remaining fields.

## Typography And Modifiers

Terminal typography means text case, weight, underline, and symbols rather than font families.

- Uppercase: application identity and primary section labels.
- Lowercase: statuses, filters, metadata labels, and control descriptions.
- Bold: selected row text, active primary navigation, focused section headers, and exceptional warnings.
- Underline: restrained active-state reinforcement for secondary controls; never the only active cue.
- Dim or muted color: secondary information, not critical instructions or errors.
- Selection markers: `▸` for the selected row and `│` on its continuation lines.
- Expansion markers: `▾` expanded and `▸` collapsed.
- Ellipsis: `…` indicates terminal-width truncation or pending CI. Truncated text must fit the requested display-cell width.

Do not use blinking text. Do not rely on italics because terminal support varies. Bold and underline must remain understandable when a terminal ignores either modifier.

## Semantic Color Roles

Render code refers to semantic roles, never literal palette values.

| Role | Meaning | Typical use |
| --- | --- | --- |
| `background` | Page and overlay surface | Full terminal, cleared modal |
| `normal` | Primary readable text | Titles, body, repository |
| `muted` | Secondary noncritical text | Labels, ages, inactive navigation |
| `muted_key` | Discoverable control or metadata key | Footer keys, file paths |
| `accent` | Product identity and active navigation | `GH-VIEW`, active tab, selection marker |
| `rule` | Nonessential separation | Unfocused horizontal/vertical rules |
| `focus_rule` | Meaningful focus or modal boundary | Active detail pane, overlay border |
| `selection` | Popup/list selection surface | Theme picker and overlay rows only |
| `success` | Successful state | Approved, passing CI, added diff |
| `info` | Neutral actionable state | Needs review, merged state |
| `warning` | Attention without failure | Changes requested, stale age, pending CI |
| `danger` | Failure or destructive state | Failed CI, load failure, removed diff |
| `reviewer` | GitHub identity | Users and teams |
| `branch` | Git references | Head/base branch names |

Every palette defines an intentional background. Informational text should target at least 4.5:1 contrast against that background; meaningful focus boundaries should target at least 3:1. Quiet rules may be lower contrast only when structure remains understandable without them. Selection must preserve readable foregrounds for every role rendered on it.

## Notation

### Identity

- GitHub users and teams use `@identity`: `@octocat`, `@owner/core-team`.
- Repository identity uses `owner/repo` where ambiguity is possible. A short `repo` name may be used only when the owner is already clear or space requires it.
- PR identity uses `#42`; repository-qualified identity uses `owner/repo #42`.
- Branches use `branch: feature/name` unless Nerd Font mode replaces only the `branch:` prefix.

### Counts And Position

- Quantities use square brackets: `MY PRS [2]`, `all [7]`, `[6 PRs]`.
- Include the noun when the surrounding label does not establish it: `[1 PR]`, `[6 PRs]`.
- Fractions represent navigation position, never quantities: `page 1/2`, discussion `1/4`.
- A filtered count must say what it counts. Counts from overlapping categories must not imply that their sum equals the total.

### Status

- Status text is lowercase: `approved`, `needs review`, `changes requested`, `draft`.
- CI uses a textual prefix and non-color symbol: `ci✓`, `ci×`, `ci…`, `ci-`.
- Stale age uses both a symbol and color: `!12d`.
- Review resolution is explicit: `thread · resolved`, `thread · unresolved`.
- Loading, empty, error, and degraded states use concise sentences, not bare placeholders such as `none`.

### Metadata

- Labels use lowercase followed by a colon: `review:`, `branch:`, `state:`, `merge:`.
- Use `requested` for a request attached to an identity. Reserve `needs review` for an aggregate PR status once its exact semantic definition is settled.
- Prefer `open PR` over ambiguous `open in browser` when the destination is the PR rather than a selected discussion item.

## Selection, Focus, And Active State

### Dashboard

- Selection uses an accent gutter marker and optional bold text.
- Dashboard rows never use a selection background.
- Continuation lines use a vertical gutter so a three-line PR remains one selected object.
- Repository expansion and selected-row state are separate signals.

### Detail

- The active pane has a focus rule and stronger section heading.
- Focus must have a structural cue in addition to color. A future focus marker may be added if palette validation shows the rule is insufficient.
- Code-line highlighting may use the selection background because it is a local selected subregion, not a dashboard row.

### Overlays

- Search and theme picker are modal overlays with a visible focus border.
- Popup/list selection may use the semantic selection background and should retain a gutter marker.
- Underlying mouse regions may remain stored, but modal input handling must prevent their activation.

## Responsive Priorities

Width is allocated in this order.

### Dashboard PR Row

1. Selection gutter and PR number/title.
2. Compact CI state.
3. Aggregate review status.
4. Age.
5. Branch and reviewer continuation metadata.

At wide widths, age and CI remain right aligned. Unknown or external status strings are truncated before they can move columns. At narrow widths, omit age and aggregate status before truncating the PR identity to nothing.

### Repository Row

1. Selection and expansion markers.
2. Repository name.
3. PR quantity.
4. Page fraction.

Truncate repository names with an ellipsis. Omit page metadata and then quantity if they cannot coexist with a useful repository label.

### Detail

1. Back path and PR identity.
2. PR title.
3. author, review, and CI.
4. branch, state, and mergeability.
5. description/discussion body and code context.

Long titles, repositories, branches, usernames, team names, reviewers, file paths, errors, and body tokens require explicit display-cell budgets. Clipping by the terminal is not a truncation strategy.

### Footer

Preserve, in order: escape/back or quit, movement, primary navigation, current-selection action, then infrequent actions. Controls that do not fit are omitted as complete key-label pairs. Never leave a key without its label. Status feedback should remain visible without displacing every escape/navigation control.

### Tiny Terminals

- Remove decorative art and secondary metadata first.
- Keep at least one visible line explaining the current mode.
- Do not allow an invisible modal to retain keyboard focus.
- If essential interaction cannot fit, show a concise `Terminal too small` state with the minimum useful dimensions.

## Interaction Rules

### Keyboard

- Shortcuts are mnemonic where possible: `/` search, `t` theme, `f` filter, `r` refresh, `b` browser, `c` copy.
- `j`/`k` and arrow keys move or scroll consistently.
- `n`/`p` and left/right move through peer items or pages.
- `enter` activates the selected item.
- `esc` exits the current mode; on the dashboard it quits.
- `q` exits or goes back where documented.
- Modal input takes precedence over page shortcuts.
- Every shortcut appears in the README. Contextual footer hints are a subset, not the complete reference.

### Mouse

- Hit targets are produced during rendering from final rectangles and visible rows.
- Scrolling, clipping, overlay position, and terminal origin are reflected in hit geometry.
- Targets never extend into footer or border cells unless those cells visibly represent the action.
- Later modal regions take precedence over underlying page regions.
- Keyboard behavior remains complete; mouse input is an alternative, not a requirement.

## Component Inventory

This inventory names recurring presentations without requiring component objects.

| Component | Example | Rules |
| --- | --- | --- |
| App header | `GH-VIEW  @octocat` | Accent product name, muted identity, optional right notice |
| Primary navigation | `1 MY PRS [2]    2 AWAITING REVIEW [7]` | Uppercase, accent active item, exact clickable label geometry |
| Secondary filter | `all [7]   direct [6]   team [2]` | Lowercase, muted, active emphasis; compact active fallback on narrow widths |
| Repository row | `▾ owner/repo   [6 PRs]   page 1/2` | Bold repository, bracketed quantity, fractional page position |
| PR row | `needs review  #42 Fix parser   !12d   ci×` | Three-line compact unit; title truncates; age/CI align when present |
| Identity list | `requested: @octocat, @owner/core` | Every user/team prefixed with `@`; state needs a no-color cue |
| Section header | `DESCRIPTION`, `DISCUSSION  1/4` | Uppercase label; navigation fraction remains unbracketed |
| Footer | `q quit   j/k move   / search` | Lowercase, muted, prioritized whole controls |
| Search overlay | `Search PRs  / parser` | Modal focus border, visible query, bounded result rows and footer |
| Theme picker | `▸ Tokyo Night  clean neon city contrast` | Live preview, popup selection background, save/cancel hints |
| Loading state | `Loading PRs...` | Name the region being loaded; animation is supplementary |
| Empty state | `No direct review requests.` | State the empty scope; do not duplicate messages |
| Error state | `GitHub CLI is not authenticated.` | Problem first, action second, technical detail last |

## State Language

### Loading

- Initial dashboard load may replace the page with `Loading PRs...`.
- Independent detail and discussion loads should identify their region.
- Animation is not sufficient text by itself.

### Empty

- Empty text names the active scope: `No PRs opened by you.`, `No PRs awaiting your review.`, `No direct review requests.`, or `No team review requests.`
- Do not show both a section placeholder and a global empty message.

### Error

- Put the failure before decorative art or secondary detail.
- Provide the next action: install/authenticate `gh`, check GitHub status, or press `r` to retry.
- Use danger or warning emphasis for the concise problem and normal text for remediation.

### Degraded

- When stale dashboard data remains after a refresh failure, keep it understandable as stale and surface the error.
- When optional review-thread context fails, retain available PR detail and issue comments and identify the failed region once.
- Unknown GitHub values are bounded and displayed as unknown rather than breaking alignment.

## Accessibility And No-Color Requirements

- Selection, focus, stale age, CI state, diff kind, and resolution must each have a text, symbol, or structural cue.
- Reviewer outcomes currently need explicit no-color notation; color alone is insufficient.
- Instructions and errors may not use a low-contrast decorative role.
- Validate dark and light palettes independently, including text rendered on selection backgrounds.
- Use Unicode display-cell width for fitting. Do not assume one Rust `char` equals one terminal cell.
- Symbols must have an ASCII or textual interpretation. Nerd Font glyphs remain opt-in.
- The application must remain operable with keyboard only and understandable in monochrome.

## Terminology

Use these terms consistently in UI, README, tests, and documentation:

| Concept | Preferred term |
| --- | --- |
| PRs authored by the current user | `My PRs` / `opened by you` |
| PRs requesting the user's attention | `Awaiting Review` pending the open question below |
| Request attached directly to current user | `direct review request` |
| Request attached to a team | `team review request` |
| All/direct/team selector | `review filter` in documentation, `filter` in compact footer text |
| GitHub user or team | `identity` generically, `user` or `team` specifically |
| Browser action targeting PR | `open PR` |
| Repository pagination | `repo page` |
| Discussion item navigation | `discussion` |

Avoid using `requested`, `needs review`, and `awaiting review` interchangeably. `requested` describes a concrete GitHub request; aggregate status and dashboard membership require explicit definitions.

## Current Audit

### Consistent Today

- Dashboard and detail pages use a flat layout with quiet rules.
- Dashboard selection uses a gutter rather than a row background.
- Theme picker selection uses a popup background and gutter.
- Primary navigation is uppercase; secondary controls are lowercase and muted.
- User and team labels generally use `@identity`.
- Section, filter, and repository quantities use square brackets; page/discussion positions use fractions.
- CI, stale age, diff kind, and thread resolution have non-color cues.
- Mouse targets are created from rendered geometry and clipped to the visible dashboard viewport.
- Search includes loaded PRs hidden by collapsed groups without making network calls.

### Corrected In The Initial Consistency Pass

- Truncation handles zero width and Unicode terminal-cell width.
- Dashboard rows bound long titles, repository names, branches, unknown CI text, and messages.
- Narrow PR rows remove secondary status/age before PR identity and compact CI.
- Footer controls are contextual, prioritized, and emitted only as whole pairs that fit.
- Narrow awaiting-review layouts preserve the active filter when the full filter set does not fit.
- Empty states name the active view/filter and no longer duplicate a global message.
- Search result capacity accounts for borders and fixed content; search uses a focus-level modal border.
- Detail title/repository and code-context paths are bounded.
- Theme names are truncated before column padding.

### Remaining Gaps

- Direct and team filter counts overlap without an explanation in the UI.
- The Awaiting Review primary count is total loaded PRs even when a narrower filter is active.
- Reviewer results still depend too heavily on color; long lists are bounded but do not summarize omitted identities.
- Repository rows use the short repository name even when identical names from different owners need disambiguation.
- Detail metadata and body wrapping need explicit responsive budgets, especially for long branches and unbroken tokens.
- Search may show the same PR once per dashboard section and may hide the field that caused a fuzzy match.
- Search and theme picker need dedicated tiny-height behavior and list scrolling.
- Background refresh replaces loaded dashboard content; stale-data failure feedback is incomplete for some error classes.
- Selection and scroll can diverge, allowing keyboard selection to move outside the visible viewport.
- Full-page errors can preserve decorative art while clipping remediation on short terminals.
- Several palette roles, especially muted and light-theme semantic colors, need formal contrast validation.
- There is no broad visual regression suite; current render tests focus mainly on mouse geometry.

## Open Questions

Each question requires an explicit product decision before a behavior-changing implementation.

### 1. Awaiting Review Count

- Option A: keep the primary count as total loaded PRs.
- Option B: show the active filtered count.
- Option C: show `[filtered/total]`.
- **Accepted:** Option B. The primary count shows the active filtered collection. The adjacent filter gives total/direct/team counts, and fractions remain reserved for navigation state.

### 2. Direct And Team Overlap

- Option A: retain overlapping categories and document that a PR may appear in both.
- Option B: make `team` mean team-only, excluding direct requests.
- Option C: replace filters with explicit inclusive labels such as `direct [6]` and `via team [2]` plus an overlap hint.
- **Recommendation:** Option A with concise documentation/help. It follows GitHub's data and avoids hiding a team request merely because the user was also requested directly.

### 3. Primary Review Terminology

- Option A: retain `Awaiting Review`.
- Option B: use `Review Requests`.
- Option C: use `Needs Your Review`.
- **Recommendation:** Option B. It corresponds most closely to the loaded GitHub relationship and avoids claiming that every listed PR still needs action. Treat this as a product rename, not a cleanup.

### 4. Aggregate `needs review`

- Option A: use it whenever a non-draft PR is not approved or changes-requested.
- Option B: use it only for GitHub `REVIEW_REQUIRED`.
- Option C: derive it from outstanding direct/team reviewer requests.
- **Recommendation:** Option B. It is a repository-policy decision from GitHub and avoids inventing status from incomplete reviewer data. Use `review pending` or `no decision` for other states if needed.

### 5. Global Search And Duplicates

- Option A: preserve one result per section.
- Option B: deduplicate by `owner/repo #number` and show all memberships.
- Option C: search only the active view/filter.
- **Recommendation:** Option B. Search remains global and useful for collapsed groups without presenting an identical PR twice.

### 6. Search Result Scope

- Option A: opening a result preserves the active filter even if the PR is excluded from it.
- Option B: switch Awaiting Review to `all` when needed.
- Option C: keep the scope and show a temporary explanation on return.
- **Recommendation:** Option B. Returning to a view that visibly contains the opened result is the least surprising behavior.

### 7. Dashboard Mouse Activation

- Option A: retain select-first, click-selected-to-activate behavior.
- Option B: single-click activates rows immediately.
- Option C: single-click selects and a true timed double-click activates.
- **Recommendation:** Option A for now. It is safe in a TUI and matches keyboard selection plus Enter, but it should be documented explicitly if retained.

### 8. Refresh Presentation

- Option A: every refresh uses the full loading page.
- Option B: only initial load is full-screen; refresh keeps stale rows and shows progress.
- **Recommendation:** Option B. It preserves context and makes degraded refresh failures understandable.

### 9. Detail Browser Action

- Option A: `b` always opens the PR and is labeled `open PR`.
- Option B: `b` opens the selected discussion URL when discussion is focused.
- Option C: provide separate PR and discussion shortcuts.
- **Recommendation:** Option A until a second shortcut has a strong use case. The destination is predictable and the label removes ambiguity.

### 10. Detail Narrow Layout

- Option A: retain side-by-side discussion and code context at all widths.
- Option B: stack code context below discussion.
- Option C: hide code context below a breakpoint and expose a toggle.
- **Recommendation:** Option B. It preserves information and avoids adding a mode, subject to a prototype confirming acceptable vertical navigation.

### 11. Tiny-Terminal Policy

- Option A: progressively strip content with no hard minimum.
- Option B: show `Terminal too small` below a documented minimum.
- **Recommendation:** Option B after establishing useful minima for dashboard, detail, and overlays. Invisible modes are worse than an explicit limitation.

### 12. Palette Contrast Target

- Option A: require 4.5:1 for all informational text and 3:1 for focus boundaries.
- Option B: permit lower contrast for secondary text.
- **Recommendation:** Option A. `muted` frequently carries controls and status, so treating it as decorative is unsafe. Quiet nonessential rules may remain exempt.

### 13. Error-Page Controls

- Option A: instructions in the error body are sufficient.
- Option B: retain a minimal `r retry   q quit` footer and mouse retry target.
- **Recommendation:** Option B. Recovery controls should remain stable across normal and error states.

## Implementation Roadmap

### P0: Correctness And Understandability

1. Keep selected dashboard rows visible and clamp stored scroll after data, filter, page, and resize changes.
2. Ensure search and theme overlays always have a visible tiny-terminal mode or refuse to open.
3. Surface every classified refresh error while retaining loaded data.
4. Add no-color reviewer outcome notation and bound reviewer identity lists.
5. Validate and repair informational text and focus contrast in every dark and light palette.

### P1: Responsive Consistency

1. Give detail metadata explicit field priorities and display-cell budgets.
2. Resolve the narrow discussion/code-context layout question and test representative widths.
3. Add theme-picker list scrolling and compact preview/help variants.
4. Prioritize problem/remediation over art on short error pages and add a minimal error footer.
5. Make search query tail/cursor visible and show why a result matched.

### P2: Product Semantics

1. Decide primary review terminology and aggregate status semantics.
2. Decide filtered primary counts and document direct/team overlap.
3. Deduplicate global search according to the selected identity policy.
4. Decide refresh and dashboard mouse activation behavior.
5. Decide whether discussion URLs need a separate browser action.

### P3: Regression Coverage

1. Add a small number of Ratatui `TestBackend` assertions for dashboard and detail at wide, 80-column, narrow, and tiny sizes.
2. Test long repository, branch, user, team, title, reviewer, path, and unknown-status strings.
3. Test exact rendered mouse geometry after scrolling, clipping, overlays, and nonzero origins.
4. Add deterministic palette contrast tests against background and selection surfaces.
5. Prefer focused buffer assertions over a large snapshot framework until update volume demonstrates clear value.
