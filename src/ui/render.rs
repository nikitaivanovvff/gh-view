use crate::app::{App, AppView, DetailPane, DetailStatus, DiscussionStatus, Row};
use crate::github::PullRequestSource;
use crate::model::{CodeLineKind, DiscussionItem, DiscussionKind, PullRequest};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::io;
use std::time::Duration;

pub fn run(client: Box<dyn PullRequestSource>) -> Result<()> {
    let mut app = App::new(client);
    app.refresh();

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, &mut app);
    restore_terminal()?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(io::stdout(), EnterAlternateScreen).context("failed to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(io::stdout())).context("failed to create terminal")
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen).context("failed to leave alternate screen")
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.poll_background();
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.view {
                AppView::Dashboard => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Char(' ') | KeyCode::Char('o') => app.toggle_selected_group(),
                    KeyCode::Char('r') => app.refresh(),
                    KeyCode::Char('b') => app.open_selected_in_browser(),
                    KeyCode::Enter => app.open_selected_detail(),
                    _ => {}
                },
                AppView::Detail => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.back_to_dashboard(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_active_down(),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_active_up(),
                    KeyCode::Tab => app.toggle_detail_pane(),
                    KeyCode::Char('d') => app.focus_description(),
                    KeyCode::Char('D') => app.focus_discussion(),
                    KeyCode::Char('b') => app.open_selected_in_browser(),
                    KeyCode::Char('n') | KeyCode::Right => app.next_discussion(),
                    KeyCode::Char('p') | KeyCode::Left => app.previous_discussion(),
                    KeyCode::Char('r') => app.open_selected_detail(),
                    _ => {}
                },
            }
        }
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    if app.view == AppView::Detail {
        render_detail(frame, app);
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

    lines.push(Line::from(vec![
        Span::styled("GH-VIEW", theme::accent().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            app.current_user
                .as_deref()
                .map(|user| format!("@{user}"))
                .unwrap_or_else(|| "@loading".to_owned()),
            theme::muted(),
        ),
    ]));
    lines.push(rule_line(width));
    for (index, row) in rows.iter().enumerate() {
        match row {
            Row::Section(title) => {
                if *title == "Awaiting Review" {
                    lines.push(Line::raw(""));
                }
                lines.extend(section_lines(title, section_count(&rows, index), width));
            }
            Row::Group { repo, count, open } => {
                lines.push(group_line(index == app.selected, repo, *count, *open));
            }
            Row::Pr(pr) => {
                lines.push(pr_line(index == app.selected, pr, width));
                lines.push(reviewers_line(pr));
            }
            Row::Message(message) => {
                lines.push(message_line(index == app.selected, message));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines).style(theme::normal()), chunks[0]);

    let footer = vec![
        rule_line(width),
        Line::from(vec![
            Span::styled("j/k", theme::muted_key()),
            Span::styled(" move   ", theme::muted()),
            Span::styled("enter", theme::muted_key()),
            Span::styled(" details   ", theme::muted()),
            Span::styled("o", theme::muted_key()),
            Span::styled(" toggle group   ", theme::muted()),
            Span::styled("/", theme::muted_key()),
            Span::styled(" search   ", theme::muted()),
            Span::styled("r", theme::muted_key()),
            Span::styled(" refresh   ", theme::muted()),
            Span::styled("q", theme::muted_key()),
            Span::styled(" quit", theme::muted()),
        ]),
    ];
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

fn render_detail(frame: &mut ratatui::Frame<'_>, app: &mut App) {
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
    let Some(detail) = &app.detail else {
        frame.render_widget(
            Paragraph::new(vec![message_line(
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
    summary.push(rule_line(width));
    summary.push(Line::from(vec![Span::styled(
        pr.title.clone(),
        theme::normal().add_modifier(Modifier::BOLD),
    )]));
    let review_status = pr_status(pr);
    summary.push(Line::from(vec![
        Span::styled(format!("@{}", pr.author), theme::reviewer()),
        Span::styled("  review: ", theme::muted()),
        Span::styled(review_status.clone(), status_style(&review_status)),
        Span::styled("  branch: ", theme::muted()),
        Span::styled(detail.head_ref.clone(), theme::normal()),
        Span::styled(" → ", theme::muted()),
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
        Span::styled(
            format!("  {}", ci_text(pr.check_status.as_deref())),
            ci_style(pr.check_status.as_deref()),
        ),
    ]));
    if let Some(line) = loading_status_line(app) {
        summary.push(line);
    }
    summary.push(rule_line(width));

    if app.detail_status == DetailStatus::Loading {
        summary.push(Line::styled("loading PR page…", theme::muted()));
    } else {
        summary.push(Line::styled(
            "DESCRIPTION",
            theme::muted().add_modifier(Modifier::BOLD),
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

    app.detail_scroll = app
        .detail_scroll
        .min(max_scroll(summary.len(), chunks[0].height));
    frame.render_widget(
        Paragraph::new(summary)
            .style(theme::normal())
            .scroll((app.detail_scroll, 0)),
        chunks[0],
    );

    render_discussion(frame, chunks[1], app);

    let footer = vec![
        rule_line(width),
        Line::from(vec![
            Span::styled("j/k", theme::muted_key()),
            Span::styled(format!(" {}   ", active_pane_label(app)), theme::muted()),
            Span::styled("tab", theme::muted_key()),
            Span::styled(" switch   ", theme::muted()),
            Span::styled("d/D", theme::muted_key()),
            Span::styled(" desc/discussion   ", theme::muted()),
            Span::styled("n/p", theme::muted_key()),
            Span::styled(" discussion   ", theme::muted()),
            Span::styled("esc/q", theme::muted_key()),
            Span::styled(" back   ", theme::muted()),
            Span::styled("b", theme::muted_key()),
            Span::styled(" browser   ", theme::muted()),
            Span::styled("r", theme::muted_key()),
            Span::styled(" refresh detail", theme::muted()),
        ]),
    ];
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

fn render_discussion(frame: &mut ratatui::Frame<'_>, area: Rect, app: &mut App) {
    let Some(detail) = &app.detail else {
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
        Paragraph::new(vertical_rule_lines(panes[1].height as usize)),
        panes[1],
    );

    let index = app.selected_discussion_index();
    let total = detail.discussion.len();
    let Some(item) = detail.discussion.get(index) else {
        let message = match &app.discussion_status {
            DiscussionStatus::Error(error) => {
                format!("  could not load discussion threads: {error}")
            }
            _ => "  none".to_owned(),
        };
        let empty = vec![
            rule_line(panes[0].width as usize),
            Line::styled("DISCUSSION", theme::muted().add_modifier(Modifier::BOLD)),
            rule_line(panes[0].width as usize),
            Line::styled(message, theme::muted()),
        ];
        frame.render_widget(Paragraph::new(empty).style(theme::normal()), panes[0]);
        frame.render_widget(
            Paragraph::new(vec![
                rule_line(panes[2].width as usize),
                Line::styled("CODE CONTEXT", theme::muted().add_modifier(Modifier::BOLD)),
                rule_line(panes[2].width as usize),
                Line::styled("  waiting for discussion", theme::muted()),
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
        &app.discussion_status,
    );
    app.discussion_scroll = app
        .discussion_scroll
        .min(max_scroll(discussion.len(), panes[0].height));
    frame.render_widget(
        Paragraph::new(discussion)
            .style(theme::normal())
            .scroll((app.discussion_scroll, 0)),
        panes[0],
    );
    frame.render_widget(
        Paragraph::new(code_context_lines(item, panes[2].width as usize)).style(theme::normal()),
        panes[2],
    );
}

fn active_pane_label(app: &App) -> &'static str {
    match app.active_detail_pane {
        DetailPane::Description => "description",
        DetailPane::Discussion => "discussion",
    }
}

fn max_scroll(line_count: usize, height: u16) -> u16 {
    line_count
        .saturating_sub(height as usize)
        .min(u16::MAX as usize) as u16
}

fn loading_status_line(app: &App) -> Option<Line<'static>> {
    let detail_loading = app.detail_status == DetailStatus::Loading;
    let discussion_loading = app.discussion_status == DiscussionStatus::Loading;

    match (&app.detail_status, &app.discussion_status) {
        (DetailStatus::Error(error), _) => Some(Line::styled(
            format!("could not load PR details: {error}"),
            theme::danger(),
        )),
        (_, DiscussionStatus::Error(error)) => Some(Line::styled(
            format!("could not load discussion threads: {error}"),
            theme::danger(),
        )),
        _ if detail_loading && discussion_loading => Some(Line::styled(
            "loading PR details and discussion…",
            theme::muted(),
        )),
        _ if detail_loading => Some(Line::styled("loading PR details…", theme::muted())),
        _ if discussion_loading => {
            Some(Line::styled("loading discussion threads…", theme::muted()))
        }
        _ => None,
    }
}

fn vertical_rule_lines(height: usize) -> Vec<Line<'static>> {
    (0..height)
        .map(|_| Line::from(Span::styled("│", theme::rule())))
        .collect()
}

fn discussion_lines(
    item: &DiscussionItem,
    index: usize,
    total: usize,
    width: usize,
    discussion_status: &DiscussionStatus,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut label = match &item.kind {
        DiscussionKind::IssueComment => "comment".to_owned(),
        DiscussionKind::ReviewThread { resolved } if *resolved => "thread · resolved".to_owned(),
        DiscussionKind::ReviewThread { .. } => "thread · unresolved".to_owned(),
    };
    match discussion_status {
        DiscussionStatus::Loading => label.push_str(" · loading threads…"),
        DiscussionStatus::Error(_) => label.push_str(" · thread load failed"),
        _ => {}
    }

    lines.push(rule_line(width));
    lines.push(Line::from(vec![
        Span::styled("DISCUSSION", theme::muted().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  {}/{}  {}", index + 1, total, label),
            theme::muted(),
        ),
    ]));
    lines.push(rule_line(width));
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

fn code_context_lines(item: &DiscussionItem, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(rule_line(width));
    lines.push(Line::styled(
        "CODE CONTEXT",
        theme::muted().add_modifier(Modifier::BOLD),
    ));
    lines.push(rule_line(width));

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

    for code in &context.lines {
        let marker = match code.kind {
            CodeLineKind::Added => "+",
            CodeLineKind::Removed => "-",
            CodeLineKind::Context => " ",
        };
        let style = match code.kind {
            CodeLineKind::Added => theme::success(),
            CodeLineKind::Removed => theme::danger(),
            CodeLineKind::Context => theme::normal(),
        };
        let number = code
            .number
            .map(|line| format!("{line:>4}"))
            .unwrap_or_else(|| "    ".to_owned());
        lines.push(Line::from(vec![
            Span::styled(format!("{number} {marker} "), theme::muted()),
            Span::styled(truncate(&code.text, width.saturating_sub(8).max(1)), style),
        ]));
    }

    lines
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

fn section_lines(title: &str, count: usize, width: usize) -> Vec<Line<'static>> {
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

fn group_line(selected: bool, repo: &str, count: usize, open: bool) -> Line<'static> {
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

fn pr_line(selected: bool, pr: &PullRequest, width: usize) -> Line<'static> {
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

fn reviewers_line(pr: &PullRequest) -> Line<'static> {
    let reviewers = if pr.reviewers.is_empty() {
        Span::styled("no reviewers", theme::muted())
    } else {
        Span::styled(format!("@{}", pr.reviewers.join("  @")), theme::reviewer())
    };

    Line::from(vec![Span::raw("        "), reviewers])
}

fn message_line(selected: bool, message: &str) -> Line<'static> {
    let gutter = if selected { "▸" } else { " " };
    Line::from(vec![
        Span::styled(gutter, selected_style()),
        Span::raw(" "),
        Span::styled(message.to_owned(), theme::muted()),
    ])
}

fn section_count(rows: &[Row<'_>], section_index: usize) -> usize {
    rows.iter()
        .skip(section_index + 1)
        .take_while(|row| !matches!(row, Row::Section(_)))
        .filter_map(|row| match row {
            Row::Group { count, .. } => Some(*count),
            _ => None,
        })
        .sum()
}

fn rule_line(width: usize) -> Line<'static> {
    Line::from(Span::styled("━".repeat(width.max(1)), theme::rule()))
}

fn pr_status(pr: &PullRequest) -> String {
    if pr.is_draft {
        return "draft".to_owned();
    }

    match pr.review_decision.as_deref() {
        Some("APPROVED") => "approved".to_owned(),
        Some("CHANGES_REQUESTED") => "changes requested".to_owned(),
        Some("REVIEW_REQUIRED") => "needs review".to_owned(),
        Some("") | None => "needs review".to_owned(),
        Some(value) => value.to_ascii_lowercase().replace('_', " "),
    }
}

fn status_style(status: &str) -> Style {
    match status {
        "approved" => theme::success(),
        "needs review" => theme::info(),
        "changes requested" => theme::warning(),
        "draft" => theme::muted(),
        _ => theme::muted(),
    }
}

fn state_style(state: &str) -> Style {
    match state.to_ascii_uppercase().as_str() {
        "LOADING" => theme::muted(),
        "OPEN" => theme::success(),
        "MERGED" => theme::info(),
        "CLOSED" => theme::danger(),
        _ => theme::muted(),
    }
}

fn merge_style(mergeable: Option<&str>) -> Style {
    match mergeable.map(str::to_ascii_uppercase).as_deref() {
        Some("MERGEABLE") => theme::success(),
        Some("CONFLICTING") => theme::danger(),
        Some("UNKNOWN") | None => theme::muted(),
        _ => theme::warning(),
    }
}

fn truncate(value: &str, max_width: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_owned();
    }

    if max_width == 1 {
        return "…".to_owned();
    }

    let mut truncated: String = value.chars().take(max_width - 1).collect();
    truncated.push('…');
    truncated
}

fn ci_text(status: Option<&str>) -> String {
    match status {
        Some("passing") => "ci✓".to_owned(),
        Some("failing") => "ci×".to_owned(),
        Some("pending") => "ci…".to_owned(),
        Some(other) => format!("ci {other}"),
        None => "ci-".to_owned(),
    }
}

fn ci_style(status: Option<&str>) -> Style {
    match status {
        Some("passing") => theme::success(),
        Some("failing") => theme::danger(),
        Some("pending") => theme::warning(),
        _ => theme::muted(),
    }
}

fn age_label(updated_at: &str) -> String {
    if updated_at.starts_with("2026-06-30") {
        "today".to_owned()
    } else if updated_at.len() >= 10 {
        updated_at[5..10].to_owned()
    } else {
        "—".to_owned()
    }
}

fn selected_style() -> Style {
    theme::accent().add_modifier(Modifier::BOLD)
}

mod theme {
    use ratatui::style::{Color, Style};

    pub fn normal() -> Style {
        Style::default().fg(Color::Rgb(214, 211, 221))
    }

    pub fn muted() -> Style {
        Style::default().fg(Color::Rgb(91, 88, 103))
    }

    pub fn accent() -> Style {
        Style::default().fg(Color::Rgb(137, 81, 255))
    }

    pub fn rule() -> Style {
        Style::default().fg(Color::Rgb(48, 45, 57))
    }

    pub fn success() -> Style {
        Style::default().fg(Color::Rgb(35, 209, 139))
    }

    pub fn info() -> Style {
        Style::default().fg(Color::Rgb(86, 156, 214))
    }

    pub fn warning() -> Style {
        Style::default().fg(Color::Rgb(220, 170, 88))
    }

    pub fn danger() -> Style {
        Style::default().fg(Color::Rgb(232, 93, 117))
    }

    pub fn reviewer() -> Style {
        Style::default().fg(Color::Rgb(64, 196, 150))
    }

    pub fn muted_key() -> Style {
        Style::default().fg(Color::Rgb(116, 111, 132))
    }
}
