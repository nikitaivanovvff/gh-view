use super::Row;
use crate::github::{GhError, is_auth_error, is_github_outage_error};
use crate::model::PullRequest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStatus {
    Ready,
    MissingGh,
    Unauthenticated(String),
    GitHubOutage(String),
    Timeout(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardErrorPage {
    pub art: Vec<String>,
    pub lines: Vec<DashboardErrorLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DashboardErrorLine {
    Text(String),
    StatusPage,
}

pub(super) fn classify_refresh_error(error: anyhow::Error) -> AppStatus {
    if let Some(error) = error.downcast_ref::<GhError>() {
        return match error {
            GhError::Missing(_) => AppStatus::MissingGh,
            GhError::Unauthenticated(message) => AppStatus::Unauthenticated(message.clone()),
            GhError::GitHubOutage(message) => AppStatus::GitHubOutage(message.clone()),
            GhError::Timeout(message) => AppStatus::Timeout(message.clone()),
            GhError::Command(message) => classify_refresh_message(message.clone()),
        };
    }

    classify_refresh_message(error.to_string())
}

fn classify_refresh_message(message: String) -> AppStatus {
    if message.contains("executable file not found")
        || message.contains("No such file or directory")
        || message.contains("failed to run gh")
    {
        AppStatus::MissingGh
    } else if is_auth_error(&message) {
        AppStatus::Unauthenticated(message)
    } else if is_github_outage_error(&message) {
        AppStatus::GitHubOutage(message)
    } else if message.contains("timed out after ") {
        AppStatus::Timeout(message)
    } else {
        AppStatus::Error(message)
    }
}

pub(super) fn dashboard_error_page(
    status: &AppStatus,
    dashboard_empty: bool,
) -> Option<DashboardErrorPage> {
    if !dashboard_empty {
        return None;
    }

    match status {
        AppStatus::MissingGh => Some(DashboardErrorPage {
            art: Vec::new(),
            lines: vec![
                DashboardErrorLine::Text("GitHub CLI `gh` was not found on PATH.".to_owned()),
                DashboardErrorLine::Text(
                    "Install it, authenticate it, then press r to retry.".to_owned(),
                ),
            ],
        }),
        AppStatus::Unauthenticated(_) => Some(DashboardErrorPage {
            art: Vec::new(),
            lines: vec![
                DashboardErrorLine::Text("GitHub CLI is not authenticated.".to_owned()),
                DashboardErrorLine::Text("Run `gh auth login`, then press r to retry.".to_owned()),
            ],
        }),
        AppStatus::GitHubOutage(_) => Some(DashboardErrorPage {
            art: vec![
                "        Zzz".to_owned(),
                "     /\\_/\\".to_owned(),
                "    ( -.- )".to_owned(),
                "    / >  < \\".to_owned(),
                "GitHub cat is asleep on the deploy button".to_owned(),
            ],
            lines: vec![
                DashboardErrorLine::Text("Looks like GitHub is having a problem.".to_owned()),
                DashboardErrorLine::StatusPage,
            ],
        }),
        AppStatus::Timeout(_) => Some(DashboardErrorPage {
            art: Vec::new(),
            lines: vec![
                DashboardErrorLine::Text("GitHub is taking too long to answer.".to_owned()),
                DashboardErrorLine::Text(
                    "The last gh command was stopped. Press r to retry.".to_owned(),
                ),
            ],
        }),
        AppStatus::Error(_) => Some(DashboardErrorPage {
            art: Vec::new(),
            lines: vec![DashboardErrorLine::Text(
                "Could not load pull requests. Press r to retry.".to_owned(),
            )],
        }),
        AppStatus::Ready => None,
    }
}

pub(super) fn github_outage_rows() -> Vec<Row<'static>> {
    vec![
        Row::Message("        Zzz".to_owned()),
        Row::Message("     /\\_/\\".to_owned()),
        Row::Message("    ( -.- )".to_owned()),
        Row::Message("    / >  < \\".to_owned()),
        Row::Message("GitHub cat is asleep on the deploy button".to_owned()),
        Row::Message(String::new()),
        Row::Message("Looks like GitHub is having a problem.".to_owned()),
        Row::Message("Check https://www.githubstatus.com/ and press r to retry.".to_owned()),
    ]
}

pub fn pull_request_status(pr: &PullRequest) -> String {
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
