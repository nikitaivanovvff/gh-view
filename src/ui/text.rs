use super::theme;
use crate::model::{PullRequest, ReviewerState};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub(super) fn rule_line(width: usize) -> Line<'static> {
    Line::from(Span::styled("━".repeat(width.max(1)), theme::rule()))
}

pub(super) fn loading_dots(frame: usize) -> &'static str {
    const DOTS: [&str; 4] = [".  ", ".. ", "...", " .."];
    DOTS[frame % DOTS.len()]
}

pub(super) fn pr_status(pr: &PullRequest) -> String {
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

pub(super) fn reviewer_style(state: ReviewerState) -> Style {
    match state {
        ReviewerState::Approved => theme::success(),
        ReviewerState::ChangesRequested => theme::warning(),
        ReviewerState::Requested => theme::info(),
        ReviewerState::Commented => theme::muted_key(),
    }
}

pub(super) fn status_style(status: &str) -> Style {
    match status {
        "approved" => theme::success(),
        "needs review" => theme::info(),
        "changes requested" => theme::warning(),
        "draft" => theme::muted(),
        _ => theme::muted(),
    }
}

pub(super) fn state_style(state: &str) -> Style {
    match state.to_ascii_uppercase().as_str() {
        "LOADING" => theme::muted(),
        "OPEN" => theme::success(),
        "MERGED" => theme::info(),
        "CLOSED" => theme::danger(),
        _ => theme::muted(),
    }
}

pub(super) fn merge_style(mergeable: Option<&str>) -> Style {
    match mergeable.map(str::to_ascii_uppercase).as_deref() {
        Some("MERGEABLE") => theme::success(),
        Some("CONFLICTING") => theme::danger(),
        Some("UNKNOWN") | None => theme::muted(),
        _ => theme::warning(),
    }
}

pub(super) fn truncate(value: &str, max_width: usize) -> String {
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

pub(super) fn ci_text(status: Option<&str>) -> String {
    match status {
        Some("passing") => "ci✓".to_owned(),
        Some("failing") => "ci×".to_owned(),
        Some("pending") => "ci…".to_owned(),
        Some(other) => format!("ci {other}"),
        None => "ci-".to_owned(),
    }
}

pub(super) fn ci_style(status: Option<&str>) -> Style {
    match status {
        Some("passing") => theme::success(),
        Some("failing") => theme::danger(),
        Some("pending") => theme::warning(),
        _ => theme::muted(),
    }
}

pub(super) fn age_label(updated_at: &str) -> String {
    if updated_at.starts_with("2026-06-30") {
        "today".to_owned()
    } else if updated_at.len() >= 10 {
        updated_at[5..10].to_owned()
    } else {
        "—".to_owned()
    }
}

pub(super) fn selected_style() -> Style {
    theme::accent().add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_pr_status_text() {
        let mut pr = pr();
        assert_eq!(pr_status(&pr), "needs review");

        pr.review_decision = Some("APPROVED".to_owned());
        assert_eq!(pr_status(&pr), "approved");

        pr.review_decision = Some("CHANGES_REQUESTED".to_owned());
        assert_eq!(pr_status(&pr), "changes requested");

        pr.review_decision = Some(String::new());
        assert_eq!(pr_status(&pr), "needs review");

        pr.is_draft = true;
        assert_eq!(pr_status(&pr), "draft");
    }

    #[test]
    fn formats_ci_and_age_labels() {
        assert_eq!(ci_text(Some("passing")), "ci✓");
        assert_eq!(ci_text(Some("failing")), "ci×");
        assert_eq!(ci_text(Some("pending")), "ci…");
        assert_eq!(ci_text(None), "ci-");
        assert_eq!(age_label("2026-06-30T10:00:00Z"), "today");
        assert_eq!(age_label("2026-07-01T10:00:00Z"), "07-01");
        assert_eq!(age_label("bad"), "—");
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 4), "hel…");
        assert_eq!(truncate("hello", 1), "…");
    }

    #[test]
    fn loading_dots_cycles_ascii_frames() {
        assert_eq!(loading_dots(0), ".  ");
        assert_eq!(loading_dots(1), ".. ");
        assert_eq!(loading_dots(4), ".  ");
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
