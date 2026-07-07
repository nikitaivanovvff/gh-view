use super::theme;
use crate::app::pull_request_status;
use crate::model::{PullRequest, ReviewerState};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use std::time::{SystemTime, UNIX_EPOCH};

const STALE_DAYS: i64 = 7;

pub(super) fn rule_line(width: usize) -> Line<'static> {
    Line::from(Span::styled("━".repeat(width.max(1)), theme::rule()))
}

pub(super) fn loading_dots(frame: usize) -> &'static str {
    const DOTS: [&str; 4] = [".  ", ".. ", "...", " .."];
    DOTS[frame % DOTS.len()]
}

pub(super) fn pr_status(pr: &PullRequest) -> String {
    pull_request_status(pr)
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
    let Some(today_days) = current_days() else {
        return "—".to_owned();
    };

    age_label_for_day(updated_at, today_days)
}

pub(super) fn is_stale(updated_at: &str) -> bool {
    let Some(updated_days) = date_days(updated_at) else {
        return false;
    };
    let Some(today_days) = current_days() else {
        return false;
    };

    today_days.saturating_sub(updated_days) >= STALE_DAYS
}

pub(super) fn selected_style() -> Style {
    theme::accent().add_modifier(Modifier::BOLD)
}

fn age_label_for_day(updated_at: &str, today_days: i64) -> String {
    let Some(updated_days) = date_days(updated_at) else {
        return "—".to_owned();
    };
    if updated_days >= today_days {
        "today".to_owned()
    } else {
        duration_label(today_days.saturating_sub(updated_days))
    }
}

fn duration_label(days: i64) -> String {
    if days >= 365 {
        format!("{}y", days / 365)
    } else if days >= 30 {
        format!("{}mo", days / 30)
    } else {
        format!("{days}d")
    }
}

fn current_days() -> Option<i64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    Some((elapsed.as_secs() / 86_400) as i64)
}

fn date_days(value: &str) -> Option<i64> {
    if value.len() < 10 {
        return None;
    }

    let year = value.get(0..4)?.parse().ok()?;
    let month = value.get(5..7)?.parse().ok()?;
    let day = value.get(8..10)?.parse().ok()?;
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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
        let today = date_days("2026-07-07T10:00:00Z").unwrap();
        assert_eq!(age_label_for_day("2026-07-07T10:00:00Z", today), "today");
        assert_eq!(age_label_for_day("2026-07-01T10:00:00Z", today), "6d");
        assert_eq!(age_label_for_day("2026-06-07T10:00:00Z", today), "1mo");
        assert_eq!(age_label_for_day("2025-07-07T10:00:00Z", today), "1y");
        assert_eq!(age_label_for_day("bad", today), "—");
    }

    #[test]
    fn parses_dates_as_days_since_unix_epoch() {
        assert_eq!(date_days("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(date_days("1970-01-02T00:00:00Z"), Some(1));
        assert_eq!(date_days("bad"), None);
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
