use crate::model::{PullRequest, PullRequestDetail};
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MockErrorMode {
    /// Simulates GitHub returning a 5xx-style outage response.
    GitHubDown,
    /// Simulates a `gh` command exceeding the configured timeout.
    Timeout,
    /// Simulates a non-classified `gh` command failure.
    Generic,
    /// Simulates `gh` requiring authentication.
    Auth,
}

pub trait PullRequestSource: Send {
    fn clone_box(&self) -> Box<dyn PullRequestSource>;
    fn status(&self) -> GhStatus;
    fn is_mock(&self) -> bool {
        false
    }
    fn mock_error_mode(&self) -> Option<MockErrorMode> {
        None
    }
    fn set_mock_error_mode(&mut self, _mode: Option<MockErrorMode>) {}
    fn current_user(&self) -> Result<String>;
    fn fetch_dashboard(&self, login: &str) -> Result<(Vec<PullRequest>, Vec<PullRequest>)> {
        Ok((
            self.fetch_my_prs(login)?,
            self.fetch_review_requests(login)?,
        ))
    }
    fn fetch_my_prs(&self, login: &str) -> Result<Vec<PullRequest>>;
    fn fetch_review_requests(&self, login: &str) -> Result<Vec<PullRequest>>;
    fn fetch_pr_detail(&self, pr: &PullRequest) -> Result<PullRequestDetail>;
    fn fetch_pr_discussion(&self, _pr: &PullRequest) -> Result<Vec<crate::model::DiscussionItem>> {
        Ok(Vec::new())
    }
}

impl Clone for Box<dyn PullRequestSource> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GhStatus {
    Ready { version: String },
    Missing,
    Unauthenticated { message: String },
}
