use super::layout::{MouseLayout, MouseTarget};
use super::text::{
    age_label, ci_style, ci_text, display_width, is_stale, loading_dots, pr_status, reviewer_style,
    rule_line, selected_style, status_style, truncate,
};
use super::theme;
use crate::app::{
    App, DashboardErrorLine, DashboardErrorPage, DashboardSearchMatch, DashboardSection,
    ReviewScope, Row,
};
use crate::model::PullRequest;
use crate::ui::footer::{FooterItem, footer_lines};
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const DASHBOARD_VIEW_GAP: usize = 4;
const REVIEW_SCOPE_GAP: usize = 3;

pub(super) fn render_dashboard(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    mouse_layout: &mut MouseLayout,
) {
    if app.show_dashboard_loading_screen() {
        render_dashboard_loading(frame, app.loading_frame);
        return;
    }

    if let Some(error_page) = app.dashboard_error_page() {
        render_dashboard_error(frame, &error_page, mouse_layout);
        return;
    }

    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let rows = app.rows();
    let selected = app.dashboard.selected.min(rows.len().saturating_sub(1));
    let width = chunks[0].width as usize;
    let mut lines = Vec::new();
    let mut targets = Vec::new();

    let user = app
        .dashboard
        .current_user
        .as_deref()
        .map(|user| format!("@{user}"))
        .unwrap_or_else(|| "@unknown".to_owned());
    let header_left_width = display_width("GH-VIEW") + 2 + display_width(&user);
    let notice_width = width.saturating_sub(header_left_width);
    let (notice, notice_style) = if app.dashboard.loading {
        (
            format!("Refreshing PRs{}", loading_dots(app.loading_frame)),
            theme::warning(),
        )
    } else if let Some(message) = app.copy_notice_message() {
        (message.to_owned(), theme::branch())
    } else if let Some(message) = app.status_message() {
        (message.to_owned(), theme::danger())
    } else {
        (String::new(), theme::normal())
    };
    let notice = truncate(&notice, notice_width);
    let padding = notice_width.saturating_sub(display_width(&notice));
    let header = vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(user, theme::muted()),
        Span::raw(" ".repeat(padding)),
        Span::styled(notice, notice_style),
    ];
    lines.push(Line::from(header));
    lines.push(rule_line(width));
    for (index, row) in rows.iter().enumerate() {
        match row {
            Row::Section => {
                let line = lines.len();
                let mut x = 0usize;
                for (label_index, section) in
                    [DashboardSection::MyPrs, DashboardSection::AwaitingReview]
                        .into_iter()
                        .enumerate()
                {
                    let label = dashboard_view_label(app, section, label_index);
                    targets.push((
                        line,
                        1,
                        x,
                        label.chars().count(),
                        MouseTarget::DashboardSection(section),
                    ));
                    x += label.chars().count() + DASHBOARD_VIEW_GAP;
                }
                if app.dashboard.active_section() == DashboardSection::AwaitingReview {
                    for (scope, label, x) in review_scope_placements(app, width) {
                        targets.push((
                            line,
                            1,
                            x,
                            label.chars().count(),
                            MouseTarget::ReviewScope(scope),
                        ));
                    }
                }
                lines.extend(dashboard_view_lines(app, width));
            }
            Row::Group {
                repo,
                count,
                open,
                page,
                page_count,
                ..
            } => {
                targets.push((lines.len(), 1, 0, width, MouseTarget::DashboardRow(index)));
                lines.push(group_line(
                    index == selected,
                    repo,
                    *count,
                    *open,
                    *page,
                    *page_count,
                    width,
                ));
            }
            Row::Pr(pr) => {
                let is_selected = index == selected;
                targets.push((lines.len(), 3, 0, width, MouseTarget::DashboardRow(index)));
                lines.push(pr_line(is_selected, pr, width));
                lines.push(branch_line(is_selected, pr, app.config().nerd_fonts, width));
                lines.push(reviewers_line(is_selected, pr, width));
            }
            Row::Message(message) => {
                targets.push((lines.len(), 1, 0, width, MouseTarget::DashboardRow(index)));
                lines.push(message_line(index == selected, message, width));
            }
        }
    }

    let scroll = app
        .dashboard
        .scroll
        .min(max_scroll(lines.len(), chunks[0].height));
    for target in targets {
        register_visible_target(mouse_layout, chunks[0], target, scroll);
    }
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::normal())
            .scroll((scroll, 0)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(dashboard_footer_lines(app, width)),
        chunks[1],
    );

    if app.search_is_open() {
        render_search_overlay(frame, app, mouse_layout);
    }
}

fn max_scroll(line_count: usize, visible_height: u16) -> u16 {
    line_count.saturating_sub(visible_height as usize) as u16
}

fn register_visible_target(
    mouse_layout: &mut MouseLayout,
    content: Rect,
    placement: (usize, usize, usize, usize, MouseTarget),
    scroll: u16,
) {
    let (line, height, x, width, target) = placement;
    let top = content.y as i32 + line as i32 - scroll as i32;
    let bottom = top + height as i32;
    let visible_top = top.max(content.y as i32);
    let visible_bottom = bottom.min(content.bottom() as i32);
    let left = content.x.saturating_add(x.min(u16::MAX as usize) as u16);
    let right = left
        .saturating_add(width.min(u16::MAX as usize) as u16)
        .min(content.right());

    if visible_top < visible_bottom && left < right {
        mouse_layout.push(
            Rect::new(
                left,
                visible_top as u16,
                right - left,
                (visible_bottom - visible_top) as u16,
            ),
            target,
        );
    }
}

fn dashboard_footer_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let rows = app.rows();
    let selected = rows.get(app.dashboard.selected);
    let mut items = vec![
        FooterItem::new("q", "quit"),
        FooterItem::new("j/k", "move"),
        FooterItem::new("tab", "view"),
        FooterItem::new("/", "search"),
    ];
    if app.dashboard.active_section() == DashboardSection::AwaitingReview {
        items.push(FooterItem::new("f", "filter"));
    }
    match selected {
        Some(Row::Pr(_)) => items.extend([
            FooterItem::new("enter", "details"),
            FooterItem::new("c", "copy branch"),
            FooterItem::new("b", "open PR"),
        ]),
        Some(Row::Group { page_count, .. }) => {
            items.push(FooterItem::new("o", "toggle group"));
            if *page_count > 1 {
                items.push(FooterItem::new("n/p", "repo page"));
            }
        }
        _ => {}
    }
    items.extend([
        FooterItem::new("t", "theme"),
        FooterItem::new("r", "refresh"),
    ]);
    if app.is_mock() {
        let mode = match app.mock_error_mode() {
            Some(crate::github::MockErrorMode::GitHubDown) => "down",
            Some(crate::github::MockErrorMode::Timeout) => "timeout",
            Some(crate::github::MockErrorMode::Generic) => "error",
            Some(crate::github::MockErrorMode::Auth) => "auth",
            None => "ok",
        };
        items.extend([
            FooterItem::new("mock", format!("[{mode}]")),
            FooterItem::new("f1", "debug"),
        ]);
    }
    footer_lines(width, items)
}

fn render_search_overlay(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    mouse_layout: &mut MouseLayout,
) {
    let area = frame.area();
    if area.width < 40 || area.height < 9 {
        return;
    }

    let width = ((area.width as f32 * 0.7) as u16).clamp(40, area.width);
    let height = area.height.saturating_sub(2).clamp(3, 12);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    let inner_width = popup.width.saturating_sub(4) as usize;
    let Some(query) = app.search_query() else {
        return;
    };
    let selected = app.selected_search_index().unwrap_or_default();

    let matches = app.search_matches();
    let mut lines = Vec::new();
    let prompt = format!("/ {query}");
    lines.push(Line::from(vec![
        Span::styled("Search PRs ", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled(
            truncate(&prompt, inner_width.saturating_sub(11).max(1)),
            theme::normal(),
        ),
    ]));
    lines.push(rule_line(inner_width));

    let match_rows = popup.height.saturating_sub(6) as usize;
    if match_rows > 0 {
        if matches.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ", selected_style()),
                Span::styled("No matches", theme::muted()),
            ]));
        } else {
            let start = selected.saturating_sub(match_rows.saturating_sub(1));
            for (index, item) in matches.iter().enumerate().skip(start).take(match_rows) {
                let row = popup.y.saturating_add(1 + lines.len() as u16);
                mouse_layout.push(
                    Rect::new(
                        popup.x.saturating_add(1),
                        row,
                        popup.width.saturating_sub(2),
                        1,
                    ),
                    MouseTarget::SearchMatch(index),
                );
                lines.push(search_match_line(
                    index == selected,
                    item,
                    inner_width,
                    app.config().nerd_fonts,
                ));
            }
        }
    }

    lines.push(rule_line(inner_width));
    lines.push(Line::from(vec![
        Span::styled("enter", theme::muted_key()),
        Span::styled(" open  ", theme::muted()),
        Span::styled("esc", theme::muted_key()),
        Span::styled(" close  ", theme::muted()),
        Span::styled("↑/↓ ctrl-p/n", theme::muted_key()),
        Span::styled(" move", theme::muted()),
    ]));

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::focus_rule()),
            )
            .style(theme::background()),
        popup,
    );
}

fn search_match_line(
    selected: bool,
    item: &DashboardSearchMatch,
    width: usize,
    nerd_fonts: bool,
) -> Line<'static> {
    let gutter = if selected { "▸" } else { " " };
    let status = pr_status(&item.pr);
    let left = format!("{} #{} {}", item.pr.repo, item.pr.number, item.pr.title);
    let branch = truncate(&branch_label(&item.pr.head_ref, nerd_fonts), 24);
    let memberships = item
        .sections
        .iter()
        .map(|section| section.title())
        .collect::<Vec<_>>()
        .join(" + ");
    let right = format!("{status}  {memberships}");
    let right_width = right.chars().count().min(width.saturating_sub(4));
    let branch_width = if branch.is_empty() {
        0
    } else {
        branch.chars().count() + 1
    };
    let left_width = width.saturating_sub(right_width + branch_width + 4).max(1);
    let left = truncate(&left, left_width);
    let padding = width
        .saturating_sub(2 + left.chars().count() + branch_width + right_width)
        .max(1);

    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(
            left,
            theme::normal().add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
        Span::raw(if branch.is_empty() { "" } else { " " }),
        Span::styled(branch, theme::branch()),
        Span::raw(" ".repeat(padding)),
        Span::styled(truncate(&right, right_width), status_style(&status)),
    ])
}

fn branch_label(head_ref: &str, nerd_fonts: bool) -> String {
    if head_ref.is_empty() {
        String::new()
    } else if nerd_fonts {
        format!(" {head_ref}")
    } else {
        format!("branch: {head_ref}")
    }
}

fn render_dashboard_error(
    frame: &mut ratatui::Frame<'_>,
    page: &DashboardErrorPage,
    mouse_layout: &mut MouseLayout,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);
    let mut lines = Vec::new();
    let art_height = page.art.len() + usize::from(!page.art.is_empty() && !page.lines.is_empty());
    if art_height + page.lines.len() <= chunks[0].height as usize {
        for art_line in &page.art {
            lines.push(Line::styled(art_line.clone(), theme::muted()));
        }
        if !page.art.is_empty() && !page.lines.is_empty() {
            lines.push(Line::raw(""));
        }
    }
    for (index, line) in page.lines.iter().enumerate() {
        lines.push(match line {
            DashboardErrorLine::Text(text) if index == 0 => {
                Line::styled(text.clone(), theme::danger().add_modifier(Modifier::BOLD))
            }
            DashboardErrorLine::Text(text) => Line::styled(text.clone(), theme::normal()),
            DashboardErrorLine::StatusPage => Line::from(vec![
                Span::styled("Check ", theme::normal()),
                Span::styled(
                    "https://www.githubstatus.com/",
                    theme::accent().add_modifier(Modifier::BOLD),
                ),
                Span::styled(" and press r to retry.", theme::normal()),
            ]),
        });
    }

    let height = lines.len().min(chunks[0].height as usize) as u16;
    let top = chunks[0].y + chunks[0].height.saturating_sub(height) / 2;
    let centered = Rect {
        x: chunks[0].x,
        y: top,
        width: chunks[0].width,
        height,
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::normal())
            .alignment(Alignment::Center),
        centered,
    );
    frame.render_widget(
        Paragraph::new(footer_lines(
            area.width as usize,
            vec![FooterItem::new("r", "retry"), FooterItem::new("q", "quit")],
        )),
        chunks[1],
    );
    if chunks[1].height >= 2 {
        mouse_layout.push(
            Rect::new(chunks[1].x, chunks[1].y + 1, 7.min(chunks[1].width), 1),
            MouseTarget::DashboardRetry,
        );
    }
}

fn render_dashboard_loading(frame: &mut ratatui::Frame<'_>, loading_frame: usize) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let line = Line::from(vec![
        Span::styled("Loading PRs", theme::muted()),
        Span::styled(loading_dots(loading_frame), theme::accent()),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .style(theme::normal())
            .alignment(Alignment::Center),
        chunks[1],
    );
}

fn dashboard_view_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let active = app.dashboard.active_section();
    let mut spans = Vec::new();
    for (index, section) in [DashboardSection::MyPrs, DashboardSection::AwaitingReview]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            spans.push(Span::raw(" ".repeat(DASHBOARD_VIEW_GAP)));
        }
        let style = if section == active {
            theme::accent().add_modifier(Modifier::BOLD)
        } else {
            theme::muted()
        };
        spans.push(Span::styled(
            dashboard_view_label(app, section, index),
            style,
        ));
    }

    if active == DashboardSection::AwaitingReview {
        let mut rendered_width = dashboard_views_width(app);
        for (scope, label, x) in review_scope_placements(app, width) {
            spans.push(Span::raw(" ".repeat(x.saturating_sub(rendered_width))));
            let style = if scope == app.dashboard.review_scope() {
                theme::muted().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                theme::muted()
            };
            rendered_width = x + label.chars().count();
            spans.push(Span::styled(label, style));
        }
    }
    vec![Line::from(spans), rule_line(width)]
}

fn review_scope_placements(app: &App, width: usize) -> Vec<(ReviewScope, String, usize)> {
    let (all, direct, team) = app.dashboard.review_scope_counts();
    let labels = [
        (ReviewScope::All, format!("all [{all}]")),
        (ReviewScope::Direct, format!("direct [{direct}]")),
        (ReviewScope::Team, format!("team [{team}]")),
    ];
    let scopes_width = labels
        .iter()
        .map(|(_, label)| label.chars().count())
        .sum::<usize>()
        + REVIEW_SCOPE_GAP * labels.len().saturating_sub(1);
    if dashboard_views_width(app) + DASHBOARD_VIEW_GAP + scopes_width > width {
        let scope = app.dashboard.review_scope();
        let count = match scope {
            ReviewScope::All => all,
            ReviewScope::Direct => direct,
            ReviewScope::Team => team,
        };
        let name = match scope {
            ReviewScope::All => "all",
            ReviewScope::Direct => "direct",
            ReviewScope::Team => "team",
        };
        let label = format!("{name} [{count}]");
        let minimum_x = dashboard_views_width(app) + 1;
        if minimum_x + display_width(&label) <= width {
            return vec![(scope, label.clone(), width - display_width(&label))];
        }
        return Vec::new();
    }

    let mut x = width - scopes_width;
    labels
        .into_iter()
        .map(|(scope, label)| {
            let placement = (scope, label.clone(), x);
            x += label.chars().count() + REVIEW_SCOPE_GAP;
            placement
        })
        .collect()
}

fn dashboard_views_width(app: &App) -> usize {
    [DashboardSection::MyPrs, DashboardSection::AwaitingReview]
        .into_iter()
        .enumerate()
        .map(|(index, section)| dashboard_view_label(app, section, index).chars().count())
        .sum::<usize>()
        + DASHBOARD_VIEW_GAP
}

fn dashboard_view_label(app: &App, section: DashboardSection, index: usize) -> String {
    format!(
        "{} {} [{}]",
        index + 1,
        section.title().to_ascii_uppercase(),
        app.dashboard.section_pr_count(section)
    )
}

pub(super) fn group_line(
    selected: bool,
    repo: &str,
    count: usize,
    open: bool,
    page: usize,
    page_count: usize,
    width: usize,
) -> Line<'static> {
    let marker = if open { "▾" } else { "▸" };
    let gutter = if selected { "▸" } else { " " };
    if width < 4 {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::styled(
                truncate(&repo_display_name(repo), width.saturating_sub(1)),
                theme::normal().add_modifier(Modifier::BOLD),
            ),
        ]);
    }
    let pr_label = if count == 1 { "PR" } else { "PRs" };
    let page_label = if page_count > 1 && width >= 30 {
        format!("   page {page}/{page_count}")
    } else {
        String::new()
    };

    let count_label = if width >= 16 {
        format!("   [{count} {pr_label}]")
    } else {
        String::new()
    };
    let repo_width =
        width.saturating_sub(4 + display_width(&count_label) + display_width(&page_label));
    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(marker, theme::muted()),
        Span::raw(" "),
        Span::styled(
            truncate(&repo_display_name(repo), repo_width),
            theme::normal().add_modifier(Modifier::BOLD),
        ),
        Span::styled(count_label, theme::muted()),
        Span::styled(page_label, theme::accent()),
    ])
}

fn repo_display_name(repo: &str) -> String {
    repo.rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(repo)
        .to_owned()
}

pub(super) fn pr_line(selected: bool, pr: &PullRequest, width: usize) -> Line<'static> {
    let gutter = if selected { "▸" } else { " " };
    if width < 8 {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::styled(
                truncate(
                    &format!("#{} {}", pr.number, pr.title),
                    width.saturating_sub(1),
                ),
                theme::normal(),
            ),
        ]);
    }
    let dot = if selected { "●" } else { "○" };
    let status = pr_status(pr);
    let title = format!("#{} {}", pr.number, pr.title);
    let age = age_label(&pr.updated_at);
    let stale = is_stale(&pr.updated_at);
    let age_text = if stale { format!("!{age}") } else { age };
    let age_style = if stale {
        theme::warning().add_modifier(Modifier::BOLD)
    } else {
        theme::muted()
    };
    let ci_text = truncate(&ci_text(pr.check_status.as_ref()), 4);
    let indent = "    ";
    let show_status = width >= 52;
    let show_age = width >= 52;
    let show_ci = width >= 20;
    let status_block_width = usize::from(show_status) * 18;
    let right_width = usize::from(show_age) * 9 + if show_ci { display_width(&ci_text) } else { 0 };
    let fixed_left_width = 2 + indent.len() + 2 + status_block_width;
    let available_title_width = width.saturating_sub(fixed_left_width + right_width);
    let title = truncate(&title, available_title_width);
    let left_width = fixed_left_width + display_width(&title);
    let padding = width.saturating_sub(left_width + right_width);

    let mut spans = vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::raw(indent),
        Span::styled(dot, selected_style()),
        Span::raw(" "),
    ];
    if show_status {
        spans.push(Span::styled(
            format!("{status:<17}"),
            status_style(&status).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        title,
        theme::normal().add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }),
    ));
    spans.push(Span::raw(" ".repeat(padding)));
    if show_age {
        spans.push(Span::styled(format!("{age_text:>6}"), age_style));
        spans.push(Span::raw("   "));
    }
    if show_ci {
        spans.push(Span::styled(ci_text, ci_style(pr.check_status.as_ref())));
    }
    Line::from(spans)
}

pub(super) fn branch_line(
    selected: bool,
    pr: &PullRequest,
    nerd_fonts: bool,
    width: usize,
) -> Line<'static> {
    let gutter = if selected { "│" } else { " " };
    let indent_width = width.saturating_sub(1).min(7);
    let indent = " ".repeat(indent_width);
    if pr.head_ref.is_empty() {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::raw(indent),
        ]);
    }

    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(indent),
        Span::styled(
            truncate(
                &branch_label(&pr.head_ref, nerd_fonts),
                width.saturating_sub(1 + indent_width),
            ),
            theme::branch().add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ])
}

pub(super) fn reviewers_line(selected: bool, pr: &PullRequest, width: usize) -> Line<'static> {
    let gutter = if selected { "│" } else { " " };
    let indent_width = width.saturating_sub(1).min(7);
    let indent = " ".repeat(indent_width);
    let completed_reviewers: Vec<_> = pr
        .reviewers
        .iter()
        .filter(|reviewer| reviewer.state != crate::model::ReviewerState::Requested)
        .collect();
    if pr.review_requested.is_empty() && completed_reviewers.is_empty() {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::raw(indent),
            Span::styled(
                truncate("no reviewers", width.saturating_sub(1 + indent_width)),
                theme::muted(),
            ),
        ]);
    }

    let mut spans = vec![Span::styled(gutter, selected_style()), Span::raw(indent)];
    let mut used = 1 + indent_width;
    if !pr.review_requested.is_empty() {
        if !push_fitted(&mut spans, "requested: ", theme::muted(), &mut used, width) {
            return Line::from(spans);
        }
        for (index, target) in pr.review_requested.iter().enumerate() {
            if index > 0 && !push_fitted(&mut spans, ", ", theme::normal(), &mut used, width) {
                break;
            }
            let label = match target {
                crate::model::ReviewRequestTarget::User(login) => format!("@{login}"),
                crate::model::ReviewRequestTarget::Team(team) if team.is_empty() => {
                    "your team".to_owned()
                }
                crate::model::ReviewRequestTarget::Team(team) => format!("@{team}"),
            };
            if !push_fitted(&mut spans, &label, theme::reviewer(), &mut used, width) {
                break;
            }
        }
    }
    for (index, reviewer) in completed_reviewers.iter().enumerate() {
        if index == 0
            && !pr.review_requested.is_empty()
            && !push_fitted(&mut spans, "  ", theme::normal(), &mut used, width)
        {
            break;
        }
        if index > 0 && !push_fitted(&mut spans, "  ", theme::normal(), &mut used, width) {
            break;
        }
        if !push_fitted(
            &mut spans,
            &format!("@{}", reviewer.login),
            reviewer_style(reviewer.state),
            &mut used,
            width,
        ) {
            break;
        }
    }

    Line::from(spans)
}

fn push_fitted(
    spans: &mut Vec<Span<'static>>,
    value: &str,
    style: ratatui::style::Style,
    used: &mut usize,
    width: usize,
) -> bool {
    let remaining = width.saturating_sub(*used);
    let fitted = truncate(value, remaining);
    let complete = fitted == value;
    *used += display_width(&fitted);
    if !fitted.is_empty() {
        spans.push(Span::styled(fitted, style));
    }
    complete
}

pub(super) fn message_line(selected: bool, message: &str, width: usize) -> Line<'static> {
    let gutter = if selected { "▸" } else { " " };
    if width < 2 {
        return Line::styled(truncate(gutter, width), selected_style());
    }
    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(truncate(message, width.saturating_sub(2)), theme::muted()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::MockGhClient;
    use crate::model::CheckStatus;
    use crate::model::ReviewerState;

    #[test]
    fn mock_footer_links_to_debug_popup_without_listing_error_states() {
        let app = App::with_default_config(Box::new(MockGhClient::new()));
        let footer = dashboard_footer_lines(&app, 500)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(footer.contains("f1 debug"));
        assert!(!footer.contains("5 down"));
        assert!(!footer.contains("6 timeout"));
        assert!(!footer.contains("7 error"));
        assert!(!footer.contains("8 auth"));
    }

    #[test]
    fn group_pr_and_message_lines_include_expected_text() {
        let pr = pr();
        let group = group_line(true, "owner/repo", 2, true, 1, 1, 80).to_string();
        let pr_line = pr_line(true, &pr, 80).to_string();
        let message = message_line(true, "No PRs", 80).to_string();

        assert!(group.contains("repo"));
        assert!(group.contains("2 PRs"));
        assert!(pr_line.contains("#1 Title"));
        assert!(pr_line.contains("no decision"));
        assert!(message.contains("No PRs"));
    }

    #[test]
    fn group_line_uses_singular_pr_label() {
        assert!(
            group_line(false, "owner/repo", 1, false, 1, 1, 80)
                .to_string()
                .contains("1 PR")
        );
    }

    #[test]
    fn group_line_shows_page_label_when_repo_has_multiple_pages() {
        assert!(
            group_line(false, "owner/repo", 12, true, 2, 3, 80)
                .to_string()
                .contains("page 2/3")
        );
        assert!(
            !group_line(false, "owner/repo", 5, true, 1, 1, 80)
                .to_string()
                .contains("page")
        );
    }

    #[test]
    fn reviewer_line_colors_reviewers_by_state() {
        let mut pr = pr();
        pr.reviewers = vec![
            crate::model::Reviewer {
                login: "alice".to_owned(),
                state: ReviewerState::Approved,
            },
            crate::model::Reviewer {
                login: "bob".to_owned(),
                state: ReviewerState::ChangesRequested,
            },
            crate::model::Reviewer {
                login: "carol".to_owned(),
                state: ReviewerState::Requested,
            },
        ];
        pr.review_requested = vec![crate::model::ReviewRequestTarget::User("carol".to_owned())];

        let line = reviewers_line(false, &pr, 80).to_string();

        assert!(line.contains("@alice"));
        assert!(line.contains("@bob"));
        assert!(line.contains("@carol"));
        assert!(!line.contains("you"));
    }

    #[test]
    fn reviewer_line_shows_requested_team() {
        let mut pr = pr();
        pr.review_requested = vec![crate::model::ReviewRequestTarget::Team(
            "owner/core-team".to_owned(),
        )];

        let line = reviewers_line(false, &pr, 80).to_string();

        assert!(line.contains("@owner/core-team"));
        assert!(!line.contains("no reviewers"));
    }

    #[test]
    fn pr_line_marks_stale_pr_age() {
        let mut pr = pr();
        pr.updated_at = "1970-01-01T00:00:00Z".to_owned();

        assert!(pr_line(false, &pr, 80).to_string().contains('!'));
    }

    #[test]
    fn compact_rows_bound_long_user_controlled_text() {
        let mut pr = pr();
        pr.title = "A very long pull request title that cannot fit".to_owned();
        pr.head_ref = "feature/with-a-very-long-branch-name".to_owned();
        pr.check_status = Some(CheckStatus::Unknown("unexpected-provider-state".to_owned()));
        pr.review_requested = vec![crate::model::ReviewRequestTarget::Team(
            "owner/a-team-with-an-extremely-long-name".to_owned(),
        )];

        for width in [1, 4, 7, 12, 20, 40, 80] {
            assert!(display_width(&pr_line(false, &pr, width).to_string()) <= width);
            assert!(display_width(&branch_line(false, &pr, false, width).to_string()) <= width);
            assert!(display_width(&reviewers_line(false, &pr, width).to_string()) <= width);
            assert!(
                display_width(
                    &group_line(
                        false,
                        "owner/an-extremely-long-repository-name",
                        123,
                        true,
                        12,
                        30,
                        width,
                    )
                    .to_string(),
                ) <= width
            );
        }
    }

    #[test]
    fn search_result_shows_all_dashboard_memberships() {
        let item = DashboardSearchMatch {
            pr: pr(),
            sections: vec![DashboardSection::MyPrs, DashboardSection::AwaitingReview],
        };

        let line = search_match_line(false, &item, 120, false).to_string();

        assert!(line.contains("My PRs + Review Requests"));
    }

    fn pr() -> PullRequest {
        PullRequest {
            repo: "owner/repo".to_owned(),
            number: 1,
            title: "Title".to_owned(),
            author: "author".to_owned(),
            head_ref: "feature-title".to_owned(),
            url: "https://example.test".to_owned(),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
            state: "OPEN".to_owned(),
            is_draft: false,
            review_decision: None,
            check_status: None,
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }
}
