use crate::model::{PullRequest, PullRequestDetail};
use anyhow::Result;

pub trait PullRequestSource: Send {
    fn clone_box(&self) -> Box<dyn PullRequestSource>;
    fn status(&self) -> GhStatus;
    fn current_user(&self) -> Result<String>;
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
