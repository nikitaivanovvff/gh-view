use super::layout::{DetailLayout, MouseLayout, MouseTarget};
use super::text::{
    age_label, ci_style, ci_text, display_width, loading_dots, merge_style, pr_status, rule_line,
    state_style, status_style, truncate,
};
use super::theme;
use crate::app::{App, DetailPane, DetailStatus, DiscussionStatus};
use crate::model::{CodeLineKind, DiscussionItem, DiscussionKind};
use crate::ui::footer::{FooterItem, footer_lines};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_segmentation::UnicodeSegmentation;

const DETAIL_STACK_BELOW_WIDTH: u16 = 96;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DiscussionLayout {
    discussion: Rect,
    code: Rect,
    separator: Option<Rect>,
}

pub(super) fn render_detail(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    mouse_layout: &mut MouseLayout,
) {
    let area = frame.area();
    let layout = DetailLayout::new(area);
    mouse_layout.push(
        layout.description,
        MouseTarget::DetailPane(DetailPane::Description),
    );
    mouse_layout.push(
        layout.discussion,
        MouseTarget::DetailPane(DetailPane::Discussion),
    );

    let width = area.width as usize;
    let Some(detail) = &app.detail.current else {
        frame.render_widget(
            Paragraph::new(vec![super::dashboard::message_line(
                false,
                "No PR detail loaded. Press esc to go back.",
                width,
            )])
            .style(theme::normal()),
            layout.description,
        );
        return;
    };

    let pr = &detail.pr;
    let mut summary = Vec::new();
    summary.push(Line::from(vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            truncate(
                &format!("{} #{}", pr.repo, pr.number),
                width.saturating_sub(9),
            ),
            theme::muted(),
        ),
    ]));
    summary.push(focus_rule_line(
        width,
        app.detail.active_pane == DetailPane::Description,
    ));
    summary.push(Line::from(vec![Span::styled(
        truncate(&pr.title, width),
        theme::normal().add_modifier(Modifier::BOLD),
    )]));
    summary.extend(metadata_lines(app, detail, width));
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

    let description_scroll = app
        .detail
        .description_scroll
        .min(max_scroll(summary.len(), layout.description.height));
    frame.render_widget(
        Paragraph::new(summary)
            .style(theme::normal())
            .scroll((description_scroll, 0)),
        layout.description,
    );

    render_discussion(frame, layout.discussion, app);

    let footer = footer_lines(
        width,
        vec![
            FooterItem::new("esc/q", "back"),
            FooterItem::new("j/k", "scroll"),
            FooterItem::new("tab", "focus"),
            FooterItem::new("n/p", "discussion"),
            FooterItem::new("?", "help"),
        ],
    );
    frame.render_widget(Paragraph::new(footer), layout.footer);
}

fn render_discussion(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let Some(detail) = &app.detail.current else {
        return;
    };

    let panes = discussion_layout(area);

    if let Some(separator) = panes.separator {
        frame.render_widget(
            Paragraph::new(vertical_rule_lines(
                separator.height as usize,
                app.detail.active_pane == DetailPane::Discussion,
            )),
            separator,
        );
    }

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
                panes.discussion.width as usize,
                app.detail.active_pane == DetailPane::Discussion,
            ),
            section_label(
                "DISCUSSION",
                app.detail.active_pane == DetailPane::Discussion,
            ),
            focus_rule_line(
                panes.discussion.width as usize,
                app.detail.active_pane == DetailPane::Discussion,
            ),
            Line::styled(message, theme::muted()),
        ];
        frame.render_widget(
            Paragraph::new(empty).style(theme::normal()),
            panes.discussion,
        );
        frame.render_widget(
            Paragraph::new(vec![
                focus_rule_line(
                    panes.code.width as usize,
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                section_label(
                    "CODE CONTEXT",
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                focus_rule_line(
                    panes.code.width as usize,
                    app.detail.active_pane == DetailPane::Discussion,
                ),
                Line::styled(code_context_message, theme::muted()),
            ])
            .style(theme::normal()),
            panes.code,
        );
        return;
    };

    let discussion = discussion_lines(
        item,
        index,
        total,
        panes.discussion.width as usize,
        &app.detail.discussion_status,
        app.detail.active_pane == DetailPane::Discussion,
    );
    let discussion_scroll = app
        .detail
        .discussion_scroll
        .min(max_scroll(discussion.len(), panes.discussion.height));
    frame.render_widget(
        Paragraph::new(discussion)
            .style(theme::normal())
            .scroll((discussion_scroll, 0)),
        panes.discussion,
    );
    frame.render_widget(
        Paragraph::new(code_context_lines(
            item,
            panes.code.width as usize,
            panes.code.height as usize,
            app.detail.active_pane == DetailPane::Discussion,
        ))
        .style(theme::normal()),
        panes.code,
    );
}

fn discussion_layout(area: Rect) -> DiscussionLayout {
    if area.width < DETAIL_STACK_BELOW_WIDTH {
        let panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        DiscussionLayout {
            discussion: panes[0],
            code: panes[1],
            separator: None,
        }
    } else {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(50),
            ])
            .split(area);
        DiscussionLayout {
            discussion: panes[0],
            separator: Some(panes[1]),
            code: panes[2],
        }
    }
}

fn max_scroll(line_count: usize, height: u16) -> u16 {
    line_count
        .saturating_sub(height as usize)
        .min(u16::MAX as usize) as u16
}

fn metadata_lines(
    app: &App,
    detail: &crate::model::PullRequestDetail,
    width: usize,
) -> Vec<Line<'static>> {
    let pr = &detail.pr;
    let review_status = pr_status(pr);
    let mut items = vec![vec![Span::styled(
        truncate(&format!("@{}", pr.author), width),
        theme::reviewer(),
    )]];

    if app.detail_is_loading() {
        items.push(vec![Span::styled(
            loading_dots(app.loading_frame),
            theme::accent(),
        )]);
    }

    items.extend([
        metadata_item(
            "review: ",
            &review_status,
            width,
            status_style(&review_status),
        ),
        vec![Span::styled(
            truncate(&ci_text(pr.check_status.as_ref()), width),
            ci_style(pr.check_status.as_ref()),
        )],
    ]);

    if app.detail.detail_status == DetailStatus::Ready {
        items.extend([
            metadata_item(
                "branch: ",
                &format!("{} -> {}", detail.head_ref, detail.base_ref),
                width,
                theme::normal(),
            ),
            metadata_item(
                "state: ",
                &detail.state.to_ascii_lowercase(),
                width,
                state_style(&detail.state),
            ),
            metadata_item(
                "merge: ",
                &detail
                    .mergeable
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_ascii_lowercase(),
                width,
                merge_style(detail.mergeable.as_deref()),
            ),
        ]);
    }

    pack_metadata_items(items, width)
}

fn metadata_item(
    label: &'static str,
    value: &str,
    width: usize,
    style: Style,
) -> Vec<Span<'static>> {
    let value = truncate(value, width.saturating_sub(display_width(label)));
    vec![
        Span::styled(label, theme::muted()),
        Span::styled(value, style),
    ]
}

fn pack_metadata_items(items: Vec<Vec<Span<'static>>>, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut used = 0;

    for item in items {
        let item_width: usize = item
            .iter()
            .map(|span| display_width(span.content.as_ref()))
            .sum();
        let separator_width = usize::from(used > 0) * 2;
        if used > 0 && used + separator_width + item_width > width {
            lines.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        if used > 0 {
            spans.push(Span::raw("  "));
            used += 2;
        }
        spans.extend(item);
        used += item_width;
    }

    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
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
        Span::styled(
            truncate(
                &format!("{}{}", context.path, line_hint),
                width.saturating_sub(2),
            ),
            theme::muted_key(),
        ),
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
    let available = width.saturating_sub(display_width(indent));
    for paragraph in text.split('\n') {
        if available == 0 {
            lines.push(Line::from(Span::raw(truncate(indent, width))));
            continue;
        }
        let wrapped = wrap_preserving_whitespace(paragraph, available);
        for content in wrapped {
            push_wrapped_line(lines, indent, content, style);
        }
    }
}

fn wrap_preserving_whitespace(value: &str, width: usize) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }

    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut token_is_whitespace = None;
    for grapheme in value.graphemes(true) {
        let is_whitespace = grapheme.chars().all(char::is_whitespace);
        if token_is_whitespace.is_some_and(|current| current != is_whitespace) {
            tokens.push(std::mem::take(&mut token));
        }
        token.push_str(grapheme);
        token_is_whitespace = Some(is_whitespace);
    }
    if !token.is_empty() {
        tokens.push(token);
    }

    let mut wrapped = Vec::new();
    let mut current = String::new();
    for token in tokens {
        let is_whitespace = token.chars().all(char::is_whitespace);
        if !is_whitespace
            && !current.is_empty()
            && current.chars().any(|character| !character.is_whitespace())
            && display_width(&current) + display_width(&token) > width
        {
            wrapped.push(std::mem::take(&mut current));
        }

        for grapheme in token.graphemes(true) {
            let grapheme_width = display_width(grapheme);
            if !current.is_empty() && display_width(&current) + grapheme_width > width {
                wrapped.push(std::mem::take(&mut current));
            }
            current.push_str(grapheme);
        }
    }
    wrapped.push(current);
    wrapped
}

fn push_wrapped_line(
    lines: &mut Vec<Line<'static>>,
    indent: &'static str,
    content: String,
    style: Style,
) {
    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(content, style),
    ]));
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
    fn status_line_reports_errors_without_loading_messages() {
        let mut app = App::with_default_config(Box::new(EmptySource));
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
    fn metadata_items_wrap_without_exceeding_terminal_width() {
        let lines = pack_metadata_items(
            vec![
                vec![Span::styled(
                    truncate("@an-extremely-long-author-name", 20),
                    theme::reviewer(),
                )],
                metadata_item("review: ", "changes requested", 20, theme::warning()),
                metadata_item(
                    "branch: ",
                    "feature/a-very-long-name -> main",
                    20,
                    theme::normal(),
                ),
            ],
            20,
        );

        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|line| display_width(&line.to_string()) <= 20)
        );
    }

    #[test]
    fn discussion_layout_stacks_below_breakpoint() {
        let stacked = discussion_layout(Rect::new(3, 5, 95, 20));
        assert_eq!(stacked.discussion, Rect::new(3, 5, 95, 10));
        assert_eq!(stacked.code, Rect::new(3, 15, 95, 10));
        assert_eq!(stacked.separator, None);

        let wide = discussion_layout(Rect::new(3, 5, 96, 20));
        assert_eq!(wide.discussion.y, wide.code.y);
        assert_eq!(wide.separator.unwrap().width, 1);
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
        assert_eq!(text, vec!["  one ", "  two ", "  three", "  ", "  four"]);
        assert!(text.iter().all(|line| display_width(line) <= 8));
    }

    #[test]
    fn push_wrapped_splits_wide_and_unbroken_tokens_by_display_width() {
        let mut lines = Vec::new();

        push_wrapped(
            &mut lines,
            "界面界面 abcdefghijklmnop",
            10,
            "  ",
            theme::normal(),
        );

        let text: Vec<_> = lines.iter().map(Line::to_string).collect();
        let reconstructed: String = text
            .iter()
            .map(|line| line.strip_prefix("  ").unwrap())
            .collect();
        assert_eq!(reconstructed, "界面界面 abcdefghijklmnop");
        assert!(text.iter().all(|line| display_width(line) <= 10));
    }

    #[test]
    fn push_wrapped_preserves_indentation_and_grapheme_clusters() {
        let mut lines = Vec::new();
        let family = "👨‍👩‍👧‍👦";
        let input = format!("  aligned   text {family}{family}");

        push_wrapped(&mut lines, &input, 12, "  ", theme::normal());

        let text: Vec<_> = lines.iter().map(Line::to_string).collect();
        let reconstructed: String = text
            .iter()
            .map(|line| line.strip_prefix("  ").unwrap())
            .collect();
        assert_eq!(reconstructed, input);
        assert!(text.first().unwrap().starts_with("    aligned"));
        assert!(text.iter().all(|line| display_width(line) <= 12));
        assert_eq!(text.iter().filter(|line| line.contains(family)).count(), 1);
        assert_eq!(
            text.iter()
                .map(|line| line.matches(family).count())
                .sum::<usize>(),
            2
        );
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
