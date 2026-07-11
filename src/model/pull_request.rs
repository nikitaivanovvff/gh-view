use super::DiscussionItem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PullRequest {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub head_ref: String,
    pub url: String,
    pub updated_at: String,
    pub state: String,
    pub is_draft: bool,
    pub review_decision: Option<String>,
    pub check_status: Option<CheckStatus>,
    pub reviewers: Vec<Reviewer>,
    pub review_requested: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckStatus {
    Passing,
    Failing,
    Pending,
    Unknown(String),
}

impl CheckStatus {
    pub fn from_label(value: impl Into<String>) -> Self {
        let value = value.into();
        match value.as_str() {
            "passing" => Self::Passing,
            "failing" => Self::Failing,
            "pending" => Self::Pending,
            _ => Self::Unknown(value),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Passing => "passing",
            Self::Failing => "failing",
            Self::Pending => "pending",
            Self::Unknown(value) => value,
        }
    }

    pub fn from_rollup_state(state: &str) -> Option<Self> {
        match state {
            "SUCCESS" => Some(Self::Passing),
            "FAILURE" | "ERROR" => Some(Self::Failing),
            "PENDING" | "EXPECTED" => Some(Self::Pending),
            "" => None,
            _ => Some(Self::Pending),
        }
    }

    pub fn summarize<'a>(checks: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        let mut checks = checks.into_iter().peekable();
        checks.peek()?;
        let mut has_pending = false;

        for check in checks {
            match check.to_ascii_uppercase().as_str() {
                "FAILURE" | "FAILED" | "ERROR" | "ACTION_REQUIRED" | "CANCELLED" | "TIMED_OUT" => {
                    return Some(Self::Failing);
                }
                "SUCCESS" | "COMPLETED" | "SKIPPED" | "NEUTRAL" => {}
                _ => has_pending = true,
            }
        }

        Some(if has_pending {
            Self::Pending
        } else {
            Self::Passing
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reviewer {
    pub login: String,
    pub state: ReviewerState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewerState {
    Requested,
    Approved,
    ChangesRequested,
    Commented,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PullRequestDetail {
    pub pr: PullRequest,
    pub body: String,
    pub state: String,
    pub mergeable: Option<String>,
    pub head_ref: String,
    pub base_ref: String,
    pub reviews: Vec<PrReview>,
    pub discussion: Vec<DiscussionItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrReview {
    pub author: String,
    pub state: String,
    pub body: String,
    pub submitted_at: String,
}
