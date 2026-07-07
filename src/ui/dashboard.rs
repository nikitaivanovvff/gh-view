use super::text::{
    age_label, ci_style, ci_text, is_stale, loading_dots, pr_status, reviewer_style, rule_line,
    selected_style, status_style, truncate,
};
use super::theme;
use crate::app::{App, DashboardErrorLine, DashboardErrorPage, DashboardSearchMatch, Row};
use crate::model::PullRequest;
use crate::ui::footer::{FooterItem, footer_lines};
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub(super) fn render_dashboard(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    if app.show_dashboard_loading_screen() {
        render_dashboard_loading(frame, app.loading_frame);
        return;
    }

    if let Some(error_page) = app.dashboard_error_page() {
        render_dashboard_error(frame, &error_page);
        return;
    }

    app.clamp_selection();

    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let rows = app.rows();
    let width = chunks[0].width as usize;
    let mut lines = Vec::new();

    let user = app
        .dashboard
        .current_user
        .as_deref()
        .map(|user| format!("@{user}"))
        .unwrap_or_else(|| "@unknown".to_owned());
    let header_left_width = "GH-VIEW".chars().count() + 2 + user.chars().count();
    let notice_width = width.saturating_sub(header_left_width);
    let notice = truncate(app.copy_notice_message().unwrap_or_default(), notice_width);
    let padding = notice_width.saturating_sub(notice.chars().count());
    let header = vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(user, theme::muted()),
        Span::raw(" ".repeat(padding)),
        Span::styled(notice, theme::branch()),
    ];
    lines.push(Line::from(header));
    lines.push(rule_line(width));
    for (index, row) in rows.iter().enumerate() {
        match row {
            Row::Section(title) => {
                if *title == "Awaiting Review" {
                    lines.push(Line::raw(""));
                }
                lines.extend(section_lines(title, section_count(&rows, index), width));
            }
            Row::Group {
                repo,
                count,
                open,
                page,
                page_count,
                ..
            } => {
                lines.push(group_line(
                    index == app.dashboard.selected,
                    repo,
                    *count,
                    *open,
                    *page,
                    *page_count,
                ));
            }
            Row::Pr(pr) => {
                let selected = index == app.dashboard.selected;
                lines.push(pr_line(selected, pr, width));
                lines.push(branch_line(selected, pr, app.nerd_fonts()));
                lines.push(reviewers_line(selected, pr));
            }
            Row::Message(message) => {
                lines.push(message_line(index == app.dashboard.selected, message));
            }
        }
    }

    app.dashboard
        .clamp_scroll(max_scroll(lines.len(), chunks[0].height));
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::normal())
            .scroll((app.dashboard.scroll, 0)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(dashboard_footer_lines(app, width)),
        chunks[1],
    );

    if app.search_is_open() {
        render_search_overlay(frame, app);
    }
}

pub(super) fn row_index_at_screen_line(rows: &[Row<'_>], y: u16, scroll: u16) -> Option<usize> {
    let mut line = 2usize;
    let y = y.saturating_add(scroll) as usize;

    for (index, row) in rows.iter().enumerate() {
        if matches!(row, Row::Section("Awaiting Review")) {
            if y == line {
                return None;
            }
            line += 1;
        }

        let height = row_screen_height(row);
        if y >= line && y < line + height {
            return match row {
                Row::Section(_) => None,
                _ => Some(index),
            };
        }
        line += height;
    }

    None
}

fn max_scroll(line_count: usize, visible_height: u16) -> u16 {
    line_count.saturating_sub(visible_height as usize) as u16
}

fn row_screen_height(row: &Row<'_>) -> usize {
    match row {
        Row::Section(_) => 2,
        Row::Group { .. } | Row::Message(_) => 1,
        Row::Pr(_) => 3,
    }
}

fn dashboard_footer_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut items = vec![
        FooterItem::new("j/k", "move"),
        FooterItem::new("enter", "details"),
        FooterItem::new("/", "search"),
        FooterItem::new("c", "copy branch"),
        FooterItem::new("b", "open in browser"),
        FooterItem::new("o", "toggle group"),
        FooterItem::new("n/p", "repo page"),
        FooterItem::new("r", "refresh"),
        FooterItem::new("q", "quit"),
    ];
    if let Some(message) = app.status_message() {
        items.push(FooterItem::new("status", message));
    }
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
            FooterItem::new("0", "ok"),
            FooterItem::new("1", "down"),
            FooterItem::new("2", "timeout"),
            FooterItem::new("3", "error"),
            FooterItem::new("4", "auth"),
        ]);
    }
    footer_lines(width, items)
}

fn render_search_overlay(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    if area.width < 8 || area.height < 3 {
        return;
    }

    let width = ((area.width as f32 * 0.7) as u16).clamp(8, area.width);
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

    let match_rows = popup.height.saturating_sub(5) as usize;
    if match_rows > 0 {
        if matches.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ", selected_style()),
                Span::styled("No matches", theme::muted()),
            ]));
        } else {
            let start = selected.saturating_sub(match_rows.saturating_sub(1));
            for (index, item) in matches.iter().enumerate().skip(start).take(match_rows) {
                lines.push(search_match_line(
                    index == selected,
                    item,
                    inner_width,
                    app.nerd_fonts(),
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
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::rule()),
        ),
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
    let right = format!("{}  {}", status, item.section);
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

fn render_dashboard_error(frame: &mut ratatui::Frame<'_>, page: &DashboardErrorPage) {
    let area = frame.area();
    let mut lines = Vec::new();
    for art_line in &page.art {
        lines.push(Line::styled(art_line.clone(), theme::muted()));
    }
    if !page.art.is_empty() && !page.lines.is_empty() {
        lines.push(Line::raw(""));
    }
    for line in &page.lines {
        lines.push(match line {
            DashboardErrorLine::Text(text) => Line::styled(text.clone(), theme::muted()),
            DashboardErrorLine::StatusPage => Line::from(vec![
                Span::styled("Check ", theme::muted()),
                Span::styled(
                    "https://www.githubstatus.com/",
                    theme::accent().add_modifier(Modifier::BOLD),
                ),
                Span::styled(" and press r to retry.", theme::muted()),
            ]),
        });
    }

    let height = lines.len().min(area.height as usize) as u16;
    let top = area.y + area.height.saturating_sub(height) / 2;
    let centered = Rect {
        x: area.x,
        y: top,
        width: area.width,
        height,
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::normal())
            .alignment(Alignment::Center),
        centered,
    );
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

pub(super) fn section_lines(title: &str, count: usize, width: usize) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", title.to_ascii_uppercase()),
                theme::normal().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("({count})"), theme::muted()),
        ]),
        rule_line(width),
    ]
}

pub(super) fn group_line(
    selected: bool,
    repo: &str,
    count: usize,
    open: bool,
    page: usize,
    page_count: usize,
) -> Line<'static> {
    let marker = if open { "▾" } else { "▸" };
    let gutter = if selected { "▸" } else { " " };
    let pr_label = if count == 1 { "PR" } else { "PRs" };
    let page_label = if page_count > 1 {
        format!("   page {page}/{page_count}")
    } else {
        String::new()
    };

    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(marker, theme::muted()),
        Span::raw(" "),
        Span::styled(
            repo_display_name(repo),
            theme::normal().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   {count} {pr_label}"), theme::muted()),
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
    let ci_text = ci_text(pr.check_status.as_deref());
    let status_width = 17;
    let indent = "    ";
    let right_width = 12;
    let fixed_left_width = 2 + indent.len() + 2 + status_width + 1;
    let available_title_width = width.saturating_sub(fixed_left_width + right_width).max(1);
    let title = truncate(&title, available_title_width);
    let left_width = fixed_left_width + title.chars().count();
    let padding = width.saturating_sub(left_width + right_width).max(1);

    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::raw(indent),
        Span::styled(dot, selected_style()),
        Span::raw(" "),
        Span::styled(
            format!("{status:<status_width$}"),
            status_style(&status).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
        Span::raw(" "),
        Span::styled(
            title,
            theme::normal().add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
        Span::raw(" ".repeat(padding)),
        Span::styled(format!("{age_text:>6}"), age_style),
        Span::raw("   "),
        Span::styled(ci_text, ci_style(pr.check_status.as_deref())),
    ])
}

pub(super) fn branch_line(selected: bool, pr: &PullRequest, nerd_fonts: bool) -> Line<'static> {
    let gutter = if selected { "│" } else { " " };
    if pr.head_ref.is_empty() {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::raw("       "),
        ]);
    }

    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw("       "),
        Span::styled(
            branch_label(&pr.head_ref, nerd_fonts),
            theme::branch().add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ])
}

pub(super) fn reviewers_line(selected: bool, pr: &PullRequest) -> Line<'static> {
    let gutter = if selected { "│" } else { " " };
    if pr.reviewers.is_empty() {
        return Line::from(vec![
            Span::styled(gutter, selected_style()),
            Span::raw("       "),
            Span::styled("no reviewers", theme::muted()),
        ]);
    }

    let mut spans = vec![Span::styled(gutter, selected_style()), Span::raw("       ")];
    for (index, reviewer) in pr.reviewers.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("@{}", reviewer.login),
            reviewer_style(reviewer.state),
        ));
    }

    Line::from(spans)
}

pub(super) fn message_line(selected: bool, message: &str) -> Line<'static> {
    let gutter = if selected { "▸" } else { " " };
    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(message.to_owned(), theme::muted()),
    ])
}

pub(super) fn section_count(rows: &[Row<'_>], section_index: usize) -> usize {
    rows.iter()
        .skip(section_index + 1)
        .take_while(|row| !matches!(row, Row::Section(_)))
        .filter_map(|row| match row {
            Row::Group { count, .. } => Some(*count),
            _ => None,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ReviewerState;

    #[test]
    fn section_group_pr_and_message_lines_include_expected_text() {
        let pr = pr();
        let section = section_lines("My PRs", 3, 12)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let group = group_line(true, "owner/repo", 2, true, 1, 1).to_string();
        let pr_line = pr_line(true, &pr, 80).to_string();
        let message = message_line(true, "No PRs").to_string();

        assert!(section.contains("MY PRS"));
        assert!(section.contains("(3)"));
        assert!(group.contains("repo"));
        assert!(group.contains("2 PRs"));
        assert!(pr_line.contains("#1 Title"));
        assert!(pr_line.contains("needs review"));
        assert!(message.contains("No PRs"));
    }

    #[test]
    fn group_line_uses_singular_pr_label() {
        assert!(
            group_line(false, "owner/repo", 1, false, 1, 1)
                .to_string()
                .contains("1 PR")
        );
    }

    #[test]
    fn group_line_shows_page_label_when_repo_has_multiple_pages() {
        assert!(
            group_line(false, "owner/repo", 12, true, 2, 3)
                .to_string()
                .contains("page 2/3")
        );
        assert!(
            !group_line(false, "owner/repo", 5, true, 1, 1)
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

        let line = reviewers_line(false, &pr).to_string();

        assert!(line.contains("@alice"));
        assert!(line.contains("@bob"));
        assert!(line.contains("@carol"));
    }

    #[test]
    fn pr_line_marks_stale_pr_age() {
        let mut pr = pr();
        pr.updated_at = "1970-01-01T00:00:00Z".to_owned();

        assert!(pr_line(false, &pr, 80).to_string().contains('!'));
    }

    #[test]
    fn section_count_sums_groups_until_next_section() {
        let pr = pr();
        let rows = vec![
            Row::Section("My PRs"),
            Row::Group {
                section: crate::app::DashboardSection::MyPrs,
                repo: "owner/a",
                count: 2,
                open: true,
                page: 1,
                page_count: 1,
            },
            Row::Pr(&pr),
            Row::Section("Awaiting Review"),
            Row::Group {
                section: crate::app::DashboardSection::AwaitingReview,
                repo: "owner/b",
                count: 3,
                open: true,
                page: 1,
                page_count: 1,
            },
        ];

        assert_eq!(section_count(&rows, 0), 2);
        assert_eq!(section_count(&rows, 3), 3);
    }

    #[test]
    fn row_index_at_screen_line_maps_rendered_dashboard_lines() {
        let pr = pr();
        let rows = vec![
            Row::Section("My PRs"),
            Row::Group {
                section: crate::app::DashboardSection::MyPrs,
                repo: "owner/a",
                count: 1,
                open: true,
                page: 1,
                page_count: 1,
            },
            Row::Pr(&pr),
            Row::Section("Awaiting Review"),
            Row::Message("  none".to_owned()),
        ];

        assert_eq!(row_index_at_screen_line(&rows, 0, 0), None);
        assert_eq!(row_index_at_screen_line(&rows, 2, 0), None);
        assert_eq!(row_index_at_screen_line(&rows, 4, 0), Some(1));
        assert_eq!(row_index_at_screen_line(&rows, 5, 0), Some(2));
        assert_eq!(row_index_at_screen_line(&rows, 7, 0), Some(2));
        assert_eq!(row_index_at_screen_line(&rows, 8, 0), None);
        assert_eq!(row_index_at_screen_line(&rows, 11, 0), Some(4));
        assert_eq!(row_index_at_screen_line(&rows, 4, 1), Some(2));
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
