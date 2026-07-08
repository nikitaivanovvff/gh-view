use super::text::{
    age_label, ci_style, ci_text, loading_dots, merge_style, pr_status, rule_line, state_style,
    status_style, truncate,
};
use super::theme;
use crate::app::{App, DetailPane, DetailStatus, DiscussionStatus};
use crate::model::{CodeLineKind, DiscussionItem, DiscussionKind};
use crate::ui::footer::{FooterItem, footer_lines};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub(super) fn render_detail(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let width = area.width as usize;
    let Some(detail) = &app.detail.current else {
        frame.render_widget(
            Paragraph::new(vec![super::dashboard::message_line(
                false,
                "No PR detail loaded. Press esc to go back.",
            )])
            .style(theme::normal()),
            chunks[0],
        );
        return;
    };

    let pr = &detail.pr;
    let mut summary = Vec::new();
    summary.push(Line::from(vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("{} #{}", pr.repo, pr.number), theme::muted()),
    ]));
    summary.push(focus_rule_line(
        width,
        app.detail.active_pane == DetailPane::Description,
    ));
    summary.push(Line::from(vec![Span::styled(
        pr.title.clone(),
        theme::normal().add_modifier(Modifier::BOLD),
    )]));
    summary.push(metadata_line(app, detail));
    if let Some(line) = status_line(app) {
        summary.push(line);
    }
    summary.push(rule_line(width));

    if app.detail.detail_status != DetailStatus::Loading {
        summary.push(section_label(
            "DESCRIPTION",
            app.detail.active_pane == DetailPane::Description,
        ));
        push_wrapped(
            &mut summary,
            if detail.body.trim().is_empty() {
                "No description."
            } else {
                &detail.body
            },
            width,
            "  ",
            theme::normal(),
        );
    }

    app.detail.description_scroll = app
        .detail
        .description_scroll
        .min(max_scroll(summary.len(), chunks[0].height));
    frame.render_widget(
        Paragraph::new(summary)
            .style(theme::normal())
            .scroll((app.detail.description_scroll, 0)),
        chunks[0],
    );

    render_discussion(frame, chunks[1], app);

    let footer = footer_lines(
        width,
        vec![
            FooterItem::new("j/k", format!("scroll {}", active_pane_label(app))),
            FooterItem::new("tab", "switch focus"),
            FooterItem::new("n/p", "discussion"),
            FooterItem::new("esc/q", "back"),
            FooterItem::new("b", "open in browser"),
            FooterItem::new("r", "refresh detail"),
        ],
    );
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

fn render_discussion(frame: &mut ratatui::Frame<'_>, area: Rect, app: &mut App) {
    let Some(detail) = &app.detail.current else {
        return;
    };

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(vertical_rule_lines(
            panes[1].height as usize,
            app.detail.active_pane == DetailPane::Discussion,
        )),
        panes[1],
    );

    let index = app.selected_discussion_index();
    let total = detail.discussion.len();
    let Some(item) = detail.discussion.get(index) else {
        let discussion_loading = app.detail.discussion_status == DiscussionStatus::Loading;
        let message = match &app.detail.discussion_status {
            DiscussionStatus::Error(error) => {
                format!("  could not load discussion threads: {error}")
            }
            DiscussionStatus::Loading => String::new(),
            _ => "  none".to_owned(),
        };
        let code_context_message = if discussion_loading {
            String::new()
        } else {
            "  no discussion selected".to_owned()
        };
        let empty = vec![
            focus_rule_line(
                panes[0].width as usize,
                app.detail.active_pane == DetailPane::Discussion,
            ),
            section_label(
                "DISCUSSION",
                app.detail.active_pane == DetailPane::Discussion,
            ),
            focus_rule_line(
                panes[0].width as usize,
                app.detail.active_pane == DetailPane::Discussion,
            ),
            Line::styled(message, theme::muted()),
        ];
        frame.render_widget(Paragraph::new(empty).style(theme::normal()), panes[0]);
        frame.render_widget(
            Paragraph::new(vec![
                focus_rule_line(
                    panes[2].width as usize,
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                section_label(
                    "CODE CONTEXT",
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                focus_rule_line(
                    panes[2].width as usize,
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                Line::styled(code_context_message, theme::muted()),
            ])
            .style(theme::normal()),
            panes[2],
        );
        return;
    };

    let discussion = discussion_lines(
        item,
        index,
        total,
        panes[0].width as usize,
        &app.detail.discussion_status,
        app.detail.active_pane == DetailPane::Discussion,
    );
    app.detail.discussion_scroll = app
        .detail
        .discussion_scroll
        .min(max_scroll(discussion.len(), panes[0].height));
    frame.render_widget(
        Paragraph::new(discussion)
            .style(theme::normal())
            .scroll((app.detail.discussion_scroll, 0)),
        panes[0],
    );
    frame.render_widget(
        Paragraph::new(code_context_lines(
            item,
            panes[2].width as usize,
            panes[2].height as usize,
            app.detail.active_pane == DetailPane::Discussion,
        ))
        .style(theme::normal()),
        panes[2],
    );
}

fn active_pane_label(app: &App) -> &'static str {
    match app.detail.active_pane {
        DetailPane::Description => "description",
        DetailPane::Discussion => "discussion",
    }
}

fn max_scroll(line_count: usize, height: u16) -> u16 {
    line_count
        .saturating_sub(height as usize)
        .min(u16::MAX as usize) as u16
}

fn metadata_line(app: &App, detail: &crate::model::PullRequestDetail) -> Line<'static> {
    let pr = &detail.pr;
    let review_status = pr_status(pr);
    let mut spans = vec![Span::styled(format!("@{}", pr.author), theme::reviewer())];

    if app.detail_is_loading() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            loading_dots(app.loading_frame),
            theme::accent(),
        ));
    }

    spans.extend([
        Span::styled("  review: ", theme::muted()),
        Span::styled(review_status.clone(), status_style(&review_status)),
        Span::styled(
            format!("  {}", ci_text(pr.check_status.as_deref())),
            ci_style(pr.check_status.as_deref()),
        ),
    ]);

    if app.detail.detail_status == DetailStatus::Ready {
        spans.extend([
            Span::styled("  branch: ", theme::muted()),
            Span::styled(detail.head_ref.clone(), theme::normal()),
            Span::styled(" -> ", theme::muted()),
            Span::styled(detail.base_ref.clone(), theme::normal()),
            Span::styled("  state: ", theme::muted()),
            Span::styled(
                detail.state.to_ascii_lowercase(),
                state_style(&detail.state),
            ),
            Span::styled("  merge: ", theme::muted()),
            Span::styled(
                detail
                    .mergeable
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_ascii_lowercase(),
                merge_style(detail.mergeable.as_deref()),
            ),
        ]);
    }

    Line::from(spans)
}

fn status_line(app: &App) -> Option<Line<'static>> {
    match (&app.detail.detail_status, &app.detail.discussion_status) {
        (DetailStatus::Error(error), _) => Some(Line::styled(
            format!("could not load PR details: {error}"),
            theme::danger(),
        )),
        (_, DiscussionStatus::Error(error)) => Some(Line::styled(
            format!("could not load discussion threads: {error}"),
            theme::danger(),
        )),
        _ => None,
    }
}

fn focus_rule_line(width: usize, focused: bool) -> Line<'static> {
    if focused {
        Line::from(Span::styled("─".repeat(width), theme::focus_rule()))
    } else {
        rule_line(width)
    }
}

fn section_label(label: &'static str, focused: bool) -> Line<'static> {
    let style = if focused {
        theme::normal().add_modifier(Modifier::BOLD)
    } else {
        theme::muted().add_modifier(Modifier::BOLD)
    };
    Line::styled(label, style)
}

fn vertical_rule_lines(height: usize, focused: bool) -> Vec<Line<'static>> {
    let style = if focused {
        theme::focus_rule()
    } else {
        theme::rule()
    };
    (0..height)
        .map(|_| Line::from(Span::styled("│", style)))
        .collect()
}

fn discussion_lines(
    item: &DiscussionItem,
    index: usize,
    total: usize,
    width: usize,
    discussion_status: &DiscussionStatus,
    focused: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut label = match &item.kind {
        DiscussionKind::IssueComment => "comment".to_owned(),
        DiscussionKind::ReviewThread { resolved } if *resolved => "thread · resolved".to_owned(),
        DiscussionKind::ReviewThread { .. } => "thread · unresolved".to_owned(),
    };
    if matches!(discussion_status, DiscussionStatus::Error(_)) {
        label.push_str(" · thread load failed");
    }

    lines.push(focus_rule_line(width, focused));
    lines.push(Line::from(vec![
        Span::styled(
            "DISCUSSION",
            if focused {
                theme::normal().add_modifier(Modifier::BOLD)
            } else {
                theme::muted().add_modifier(Modifier::BOLD)
            },
        ),
        Span::styled(
            format!("  {}/{}  {}", index + 1, total, label),
            theme::muted(),
        ),
    ]));
    lines.push(focus_rule_line(width, focused));
    lines.push(Line::from(vec![
        Span::styled(format!("@{}", item.author), theme::reviewer()),
        Span::styled(format!("  {}", age_label(&item.created_at)), theme::muted()),
    ]));
    push_wrapped(&mut lines, &item.body, width, "  ", theme::normal());

    for reply in &item.replies {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("↳ ", theme::muted()),
            Span::styled(format!("@{}", reply.author), theme::reviewer()),
            Span::styled(
                format!("  {}", age_label(&reply.created_at)),
                theme::muted(),
            ),
        ]));
        push_wrapped(&mut lines, &reply.body, width, "  ", theme::normal());
    }

    lines
}

fn code_context_lines(
    item: &DiscussionItem,
    width: usize,
    height: usize,
    focused: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(focus_rule_line(width, focused));
    lines.push(section_label("CODE CONTEXT", focused));
    lines.push(focus_rule_line(width, focused));

    let Some(context) = &item.code_context else {
        lines.push(Line::styled(
            "  no code context for this comment",
            theme::muted(),
        ));
        return lines;
    };

    let line_hint = context
        .highlighted_line
        .or(context.start_line)
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled("  ", theme::muted()),
        Span::styled(format!("{}{}", context.path, line_hint), theme::muted_key()),
    ]));

    let visible_range = visible_code_line_range(context, height.saturating_sub(lines.len()));
    let highlighted_index = highlighted_code_line_index(context);
    for (index, code) in context.lines[visible_range.clone()].iter().enumerate() {
        let highlighted = highlighted_index == Some(visible_range.start + index);
        let marker = match code.kind {
            CodeLineKind::Added => "+",
            CodeLineKind::Removed => "-",
            CodeLineKind::Context => " ",
        };
        let mut style = match code.kind {
            CodeLineKind::Added => theme::success(),
            CodeLineKind::Removed => theme::danger(),
            CodeLineKind::Context => theme::normal(),
        };
        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
        }
        let number = code
            .number
            .map(|line| format!("{line:>4}"))
            .unwrap_or_else(|| "    ".to_owned());
        let gutter = if highlighted { "│" } else { " " };
        let number_style = if highlighted {
            theme::focus_rule().add_modifier(Modifier::BOLD)
        } else {
            theme::muted()
        };
        let line_style = if highlighted {
            theme::selection()
        } else {
            Style::default()
        };
        let code_text = truncate(&code.text, width.saturating_sub(9).max(1));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{gutter}{number} {marker} "),
                number_style.patch(line_style),
            ),
            Span::styled(code_text, style.patch(line_style)),
        ]));
    }

    lines
}

fn visible_code_line_range(
    context: &crate::model::CodeContext,
    available_height: usize,
) -> std::ops::Range<usize> {
    if available_height == 0 || context.lines.len() <= available_height {
        return 0..context.lines.len();
    }

    let highlighted_index = highlighted_code_line_index(context).unwrap_or(0);
    let half = available_height / 2;
    let start = highlighted_index
        .saturating_sub(half)
        .min(context.lines.len().saturating_sub(available_height));
    start..start + available_height
}

fn highlighted_code_line_index(context: &crate::model::CodeContext) -> Option<usize> {
    let highlighted_line = context.highlighted_line?;
    if let Some(kind) = &context.highlighted_kind
        && let Some(index) = context
            .lines
            .iter()
            .position(|line| line.number == Some(highlighted_line) && &line.kind == kind)
    {
        return Some(index);
    }

    context
        .lines
        .iter()
        .position(|line| {
            line.number == Some(highlighted_line) && line.kind != CodeLineKind::Removed
        })
        .or_else(|| {
            context
                .lines
                .iter()
                .position(|line| line.number == Some(highlighted_line))
        })
}

fn push_wrapped(
    lines: &mut Vec<Line<'static>>,
    text: &str,
    width: usize,
    indent: &'static str,
    style: Style,
) {
    let available = width.saturating_sub(indent.chars().count()).max(12);
    for paragraph in text.lines() {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let next_len =
                current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
            if next_len > available && !current.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(current, style),
                ]));
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(current, style),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{GhStatus, PullRequestSource};
    use crate::model::{PullRequest, PullRequestDetail};
    use anyhow::Result;

    #[derive(Clone)]
    struct EmptySource;

    impl PullRequestSource for EmptySource {
        fn clone_box(&self) -> Box<dyn PullRequestSource> {
            Box::new(self.clone())
        }

        fn status(&self) -> GhStatus {
            GhStatus::Ready {
                version: "test".to_owned(),
            }
        }

        fn current_user(&self) -> Result<String> {
            Ok("octocat".to_owned())
        }

        fn fetch_my_prs(&self, _login: &str) -> Result<Vec<PullRequest>> {
            Ok(Vec::new())
        }

        fn fetch_review_requests(&self, _login: &str) -> Result<Vec<PullRequest>> {
            Ok(Vec::new())
        }

        fn fetch_pr_detail(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
            Ok(PullRequestDetail {
                pr: pr.clone(),
                body: String::new(),
                state: "OPEN".to_owned(),
                mergeable: None,
                head_ref: "feature".to_owned(),
                base_ref: "main".to_owned(),
                reviews: Vec::new(),
                discussion: Vec::new(),
            })
        }
    }

    #[test]
    fn max_scroll_saturates_to_visible_content() {
        assert_eq!(max_scroll(3, 10), 0);
        assert_eq!(max_scroll(10, 3), 7);
    }

    #[test]
    fn active_pane_label_tracks_focused_detail_pane() {
        let mut app = App::new(Box::new(EmptySource));

        assert_eq!(active_pane_label(&app), "description");
        app.toggle_detail_pane();
        assert_eq!(active_pane_label(&app), "discussion");
    }

    #[test]
    fn status_line_reports_errors_without_loading_messages() {
        let mut app = App::new(Box::new(EmptySource));
        assert!(status_line(&app).is_none());

        app.detail.detail_status = DetailStatus::Loading;
        app.detail.discussion_status = DiscussionStatus::Loading;
        assert!(status_line(&app).is_none());

        app.detail.detail_status = DetailStatus::Error("boom".to_owned());
        assert!(status_line(&app).unwrap().to_string().contains("boom"));
    }

    #[test]
    fn vertical_rule_lines_matches_requested_height() {
        let lines = vertical_rule_lines(3, false);

        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| line.to_string() == "│"));
    }

    #[test]
    fn push_wrapped_splits_long_text_and_preserves_blank_paragraphs() {
        let mut lines = Vec::new();

        push_wrapped(
            &mut lines,
            "one two three\n\nfour",
            8,
            "  ",
            theme::normal(),
        );

        let text = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["  one two", "  three", "  ", "  four"]);
    }

    #[test]
    fn discussion_lines_include_replies_without_loading_state() {
        let item = DiscussionItem {
            kind: DiscussionKind::ReviewThread { resolved: false },
            author: "alice".to_owned(),
            body: "Please adjust this".to_owned(),
            created_at: "2026-07-01T10:00:00Z".to_owned(),
            url: "https://example.test".to_owned(),
            replies: vec![crate::model::DiscussionReply {
                author: "bob".to_owned(),
                body: "Done".to_owned(),
                created_at: "2026-07-01T10:05:00Z".to_owned(),
            }],
            code_context: None,
        };

        let lines = discussion_lines(&item, 0, 1, 80, &DiscussionStatus::Loading, false);
        let text = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("thread · unresolved"));
        assert!(!text.contains("loading"));
        assert!(text.contains("@alice"));
        assert!(text.contains("@bob"));
    }

    #[test]
    fn code_context_lines_render_empty_and_context_cases() {
        let mut item = DiscussionItem {
            kind: DiscussionKind::IssueComment,
            author: "alice".to_owned(),
            body: "Comment".to_owned(),
            created_at: "2026-07-01T10:00:00Z".to_owned(),
            url: "https://example.test".to_owned(),
            replies: Vec::new(),
            code_context: None,
        };
        assert!(
            code_context_lines(&item, 80, 20, false)
                .iter()
                .any(|line| line.to_string().contains("no code context"))
        );

        item.code_context = Some(crate::model::CodeContext {
            path: "src/main.rs".to_owned(),
            start_line: Some(10),
            highlighted_line: Some(11),
            highlighted_kind: None,
            lines: vec![crate::model::CodeContextLine {
                number: Some(11),
                kind: CodeLineKind::Added,
                text: "let value = true;".to_owned(),
            }],
        });

        let text = code_context_lines(&item, 80, 20, true)
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("src/main.rs:11"));
        assert!(text.contains("let value = true;"));
    }

    #[test]
    fn visible_code_lines_centers_highlight_when_context_is_taller_than_pane() {
        let context = crate::model::CodeContext {
            path: "src/main.rs".to_owned(),
            start_line: Some(1),
            highlighted_line: Some(10),
            highlighted_kind: None,
            lines: (1..=20)
                .map(|line| crate::model::CodeContextLine {
                    number: Some(line),
                    kind: CodeLineKind::Context,
                    text: format!("line {line}"),
                })
                .collect(),
        };

        let visible = &context.lines[visible_code_line_range(&context, 7)];

        assert_eq!(visible.first().and_then(|line| line.number), Some(7));
        assert_eq!(visible.last().and_then(|line| line.number), Some(13));
        assert!(visible.iter().any(|line| line.number == Some(10)));
    }

    #[test]
    fn highlighted_code_line_prefers_removed_side_when_comment_targets_deleted_line() {
        let context = crate::model::CodeContext {
            path: "README.md".to_owned(),
            start_line: Some(98),
            highlighted_line: Some(102),
            highlighted_kind: Some(CodeLineKind::Removed),
            lines: vec![
                crate::model::CodeContextLine {
                    number: Some(102),
                    kind: CodeLineKind::Removed,
                    text: "removed".to_owned(),
                },
                crate::model::CodeContextLine {
                    number: Some(102),
                    kind: CodeLineKind::Context,
                    text: "kept".to_owned(),
                },
            ],
        };

        assert_eq!(highlighted_code_line_index(&context), Some(0));
    }
}
