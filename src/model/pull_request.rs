use super::DiscussionItem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PullRequest {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
    pub updated_at: String,
    pub state: String,
    pub is_draft: bool,
    pub review_decision: Option<String>,
    pub check_status: Option<String>,
    pub reviewers: Vec<String>,
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
