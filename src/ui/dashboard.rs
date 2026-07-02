use super::text::{
    age_label, ci_style, ci_text, pr_status, reviewer_style, rule_line, selected_style,
    status_style, truncate,
};
use super::theme;
use crate::app::Row;
use crate::model::PullRequest;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

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

pub(super) fn group_line(selected: bool, repo: &str, count: usize, open: bool) -> Line<'static> {
    let marker = if open { "▾" } else { "▸" };
    let gutter = if selected { "▸" } else { " " };
    let pr_label = if count == 1 { "PR" } else { "PRs" };

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
    let ci_text = ci_text(pr.check_status.as_deref());
    let status_width = 17;
    let indent = "    ";
    let right_width = 11;
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
        Span::styled(format!("{status:<status_width$}"), status_style(&status)),
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
        Span::styled(format!("{age:>5}"), theme::muted()),
        Span::raw("   "),
        Span::styled(ci_text, ci_style(pr.check_status.as_deref())),
    ])
}

pub(super) fn reviewers_line(pr: &PullRequest) -> Line<'static> {
    if pr.reviewers.is_empty() {
        return Line::from(vec![
            Span::raw("        "),
            Span::styled("no reviewers", theme::muted()),
        ]);
    }

    let mut spans = vec![Span::raw("        ")];
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
        let group = group_line(true, "owner/repo", 2, true).to_string();
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
            group_line(false, "owner/repo", 1, false)
                .to_string()
                .contains("1 PR")
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

        let line = reviewers_line(&pr).to_string();

        assert!(line.contains("@alice"));
        assert!(line.contains("@bob"));
        assert!(line.contains("@carol"));
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
            },
            Row::Pr(&pr),
            Row::Section("Awaiting Review"),
            Row::Group {
                section: crate::app::DashboardSection::AwaitingReview,
                repo: "owner/b",
                count: 3,
                open: true,
            },
        ];

        assert_eq!(section_count(&rows, 0), 2);
        assert_eq!(section_count(&rows, 3), 3);
    }

    fn pr() -> PullRequest {
        PullRequest {
            repo: "owner/repo".to_owned(),
            number: 1,
            title: "Title".to_owned(),
            author: "author".to_owned(),
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
