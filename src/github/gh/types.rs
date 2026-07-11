use crate::model::{
    CodeContext, CodeContextLine, CodeLineKind, DiscussionItem, DiscussionKind, DiscussionReply,
    PrReview, PullRequest, PullRequestDetail, Reviewer, ReviewerState,
};
use serde::Deserialize;

type PullRequestPage = (Vec<PullRequest>, Option<String>);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchPullRequest {
    repository: Repository,
    number: u64,
    title: String,
    author: Option<User>,
    #[serde(default)]
    head_ref_name: String,
    review_decision: Option<String>,
    status_check_rollup: Option<Vec<CheckRun>>,
    updated_at: String,
    #[serde(default)]
    state: String,
    is_draft: bool,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DetailPullRequest {
    number: u64,
    title: String,
    author: Option<User>,
    #[serde(default)]
    head_ref_name: String,
    review_decision: Option<String>,
    status_check_rollup: Option<Vec<CheckRun>>,
    updated_at: String,
    is_draft: bool,
    url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    state: String,
    mergeable: Option<String>,
    #[serde(default)]
    base_ref_name: String,
    #[serde(default)]
    comments: Vec<GhComment>,
    #[serde(default)]
    reviews: Vec<GhReview>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhComment {
    author: Option<User>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhReview {
    author: Option<User>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    submitted_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardSearchResponse {
    data: DashboardSearchData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardResponse {
    data: DashboardData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardData {
    my_prs: Option<DashboardSearchConnection>,
    review_requests: Option<DashboardSearchConnection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSearchData {
    search: DashboardSearchConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSearchConnection {
    #[serde(default)]
    nodes: Vec<DashboardSearchPullRequest>,
    #[serde(default)]
    page_info: PageInfo,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    #[serde(default)]
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSearchPullRequest {
    repository: Repository,
    number: u64,
    title: String,
    author: Option<User>,
    #[serde(default)]
    head_ref_name: String,
    review_decision: Option<String>,
    updated_at: String,
    #[serde(default)]
    is_draft: bool,
    url: String,
    reviews: DashboardReviewConnection,
    review_requests: DashboardReviewRequestConnection,
    commits: DashboardCommitConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardReviewConnection {
    #[serde(default)]
    nodes: Vec<DashboardReview>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardReview {
    author: Option<DashboardActor>,
    #[serde(default)]
    state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardReviewRequestConnection {
    #[serde(default)]
    nodes: Vec<DashboardReviewRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardReviewRequest {
    requested_reviewer: Option<DashboardActor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardActor {
    #[serde(default)]
    login: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "__typename")]
    typename: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardCommitConnection {
    #[serde(default)]
    nodes: Vec<DashboardCommitNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardCommitNode {
    commit: DashboardCommit,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardCommit {
    status_check_rollup: Option<DashboardStatusCheckRollup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardStatusCheckRollup {
    #[serde(default)]
    state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReviewThreadsResponse {
    data: ReviewThreadsData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadsData {
    repository: ReviewThreadsRepository,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadsRepository {
    pull_request: ReviewThreadsPullRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadsPullRequest {
    review_threads: ReviewThreadConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadConnection {
    #[serde(default)]
    nodes: Vec<ReviewThreadNode>,
    #[serde(default)]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadNode {
    is_resolved: bool,
    #[serde(default)]
    path: String,
    line: Option<u64>,
    original_line: Option<u64>,
    comments: ReviewThreadCommentConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadCommentConnection {
    #[serde(default)]
    nodes: Vec<ReviewThreadComment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewThreadComment {
    author: Option<User>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    diff_hunk: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    #[serde(default)]
    name_with_owner: String,
}

#[derive(Debug, Deserialize)]
struct User {
    #[serde(default)]
    login: String,
}

#[derive(Debug, Deserialize)]
struct CheckRun {
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

impl From<SearchPullRequest> for PullRequest {
    fn from(value: SearchPullRequest) -> Self {
        Self {
            repo: value.repository.name_with_owner,
            number: value.number,
            title: value.title,
            author: value.author.map(|author| author.login).unwrap_or_default(),
            head_ref: value.head_ref_name,
            url: value.url,
            updated_at: value.updated_at,
            state: value.state,
            is_draft: value.is_draft,
            review_decision: value.review_decision,
            check_status: summarize_checks(value.status_check_rollup.as_deref().unwrap_or(&[])),
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }
}

impl DashboardSearchResponse {
    pub(super) fn into_page(self) -> (Vec<PullRequest>, Option<String>) {
        self.data.search.into_page()
    }
}

impl DashboardResponse {
    pub(super) fn into_page(self, login: &str) -> (PullRequestPage, PullRequestPage) {
        let my_prs = self
            .data
            .my_prs
            .map(DashboardSearchConnection::into_page)
            .unwrap_or_default();
        let (reviews, review_cursor) = self
            .data
            .review_requests
            .map(DashboardSearchConnection::into_page)
            .unwrap_or_default();
        let reviews = reviews
            .into_iter()
            .filter(|pr| pr.review_requested.iter().any(|reviewer| reviewer == login))
            .collect();

        (my_prs, (reviews, review_cursor))
    }
}

impl DashboardSearchConnection {
    fn into_page(self) -> (Vec<PullRequest>, Option<String>) {
        let cursor = self
            .page_info
            .has_next_page
            .then_some(self.page_info.end_cursor)
            .flatten();
        let prs = self
            .nodes
            .into_iter()
            .map(DashboardSearchPullRequest::into_pull_request)
            .collect();
        (prs, cursor)
    }
}

impl DashboardSearchPullRequest {
    fn into_pull_request(self) -> PullRequest {
        let mut reviewers: Vec<Reviewer> = self
            .review_requests
            .nodes
            .iter()
            .filter_map(|request| request.requested_reviewer.as_ref())
            .filter_map(DashboardActor::reviewer_name)
            .map(|login| Reviewer {
                login,
                state: ReviewerState::Requested,
            })
            .collect();

        for review in self.reviews.nodes {
            let Some(actor) = review.author else {
                continue;
            };
            if actor.typename == "Bot" || actor.login.is_empty() {
                continue;
            }
            upsert_reviewer(
                &mut reviewers,
                Reviewer {
                    login: actor.login,
                    state: reviewer_state(&review.state),
                },
            );
        }
        reviewers.sort_by(|left, right| left.login.cmp(&right.login));
        reviewers.dedup_by(|left, right| left.login == right.login);

        let review_requested = self
            .review_requests
            .nodes
            .into_iter()
            .filter_map(|request| request.requested_reviewer)
            .filter(|actor| actor.typename == "User" && !actor.login.is_empty())
            .map(|actor| actor.login)
            .collect();

        PullRequest {
            repo: self.repository.name_with_owner,
            number: self.number,
            title: self.title,
            author: self.author.map(|author| author.login).unwrap_or_default(),
            head_ref: self.head_ref_name,
            url: self.url,
            updated_at: self.updated_at,
            state: "OPEN".to_owned(),
            is_draft: self.is_draft,
            review_decision: self.review_decision.filter(|value| !value.is_empty()),
            check_status: self
                .commits
                .nodes
                .first()
                .and_then(|node| node.commit.status_check_rollup.as_ref())
                .and_then(|rollup| dashboard_check_status(&rollup.state)),
            reviewers,
            review_requested,
        }
    }
}

impl DashboardActor {
    fn reviewer_name(&self) -> Option<String> {
        match self.typename.as_str() {
            "User" if !self.login.is_empty() => Some(self.login.clone()),
            "Team" if !self.name.is_empty() => Some(self.name.clone()),
            _ => None,
        }
    }
}

impl ReviewThreadsResponse {
    pub(super) fn into_page(self) -> (Vec<DiscussionItem>, Option<String>) {
        let connection = self.data.repository.pull_request.review_threads;
        let cursor = connection
            .page_info
            .has_next_page
            .then_some(connection.page_info.end_cursor)
            .flatten();
        let items = connection
            .nodes
            .into_iter()
            .filter_map(ReviewThreadNode::into_discussion_item)
            .collect();
        (items, cursor)
    }
}

impl ReviewThreadNode {
    fn into_discussion_item(self) -> Option<DiscussionItem> {
        let mut comments = self.comments.nodes.into_iter();
        let first = comments.next()?;
        let highlighted_line = self.line.or(self.original_line);
        let highlighted_kind = if self.line.is_none() && self.original_line.is_some() {
            Some(CodeLineKind::Removed)
        } else {
            None
        };
        let path = self.path;
        let lines = parse_diff_hunk(&first.diff_hunk);
        let start_line = lines.iter().find_map(|line| line.number);
        let code_context = if path.is_empty() && lines.is_empty() {
            None
        } else {
            Some(CodeContext {
                path,
                start_line,
                highlighted_line,
                highlighted_kind,
                lines,
            })
        };

        Some(DiscussionItem {
            kind: DiscussionKind::ReviewThread {
                resolved: self.is_resolved,
            },
            author: first.author.map(|author| author.login).unwrap_or_default(),
            body: first.body,
            created_at: first.created_at,
            url: first.url,
            replies: comments
                .map(|reply| DiscussionReply {
                    author: reply.author.map(|author| author.login).unwrap_or_default(),
                    body: reply.body,
                    created_at: reply.created_at,
                })
                .collect(),
            code_context,
        })
    }
}

impl DetailPullRequest {
    pub(super) fn into_detail(self, repo: String) -> PullRequestDetail {
        let pr = PullRequest {
            repo,
            number: self.number,
            title: self.title,
            author: self.author.map(|author| author.login).unwrap_or_default(),
            head_ref: self.head_ref_name.clone(),
            url: self.url,
            updated_at: self.updated_at,
            state: self.state.clone(),
            is_draft: self.is_draft,
            review_decision: self.review_decision,
            check_status: summarize_checks(self.status_check_rollup.as_deref().unwrap_or(&[])),
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        };

        PullRequestDetail {
            pr,
            body: self.body,
            state: self.state,
            mergeable: self.mergeable,
            head_ref: self.head_ref_name,
            base_ref: self.base_ref_name,
            discussion: self
                .comments
                .into_iter()
                .map(|comment| DiscussionItem {
                    kind: DiscussionKind::IssueComment,
                    author: comment
                        .author
                        .map(|author| author.login)
                        .unwrap_or_default(),
                    body: comment.body,
                    created_at: comment.created_at,
                    url: comment.url,
                    replies: Vec::new(),
                    code_context: None,
                })
                .collect(),
            reviews: self
                .reviews
                .into_iter()
                .map(|review| PrReview {
                    author: review.author.map(|author| author.login).unwrap_or_default(),
                    state: review.state,
                    body: review.body,
                    submitted_at: review.submitted_at,
                })
                .collect(),
        }
    }
}

fn dashboard_check_status(state: &str) -> Option<String> {
    match state {
        "SUCCESS" => Some("passing".to_owned()),
        "FAILURE" | "ERROR" => Some("failing".to_owned()),
        "PENDING" | "EXPECTED" => Some("pending".to_owned()),
        "" => None,
        _ => Some("pending".to_owned()),
    }
}

fn upsert_reviewer(reviewers: &mut Vec<Reviewer>, reviewer: Reviewer) {
    if let Some(existing) = reviewers
        .iter_mut()
        .find(|existing| existing.login == reviewer.login)
    {
        existing.state = reviewer.state;
    } else {
        reviewers.push(reviewer);
    }
}

fn reviewer_state(state: &str) -> ReviewerState {
    match state {
        "APPROVED" => ReviewerState::Approved,
        "CHANGES_REQUESTED" => ReviewerState::ChangesRequested,
        _ => ReviewerState::Commented,
    }
}

fn parse_diff_hunk(diff_hunk: &str) -> Vec<CodeContextLine> {
    let mut old_line = None;
    let mut new_line = None;
    let mut lines = Vec::new();

    for raw_line in diff_hunk.lines() {
        if raw_line.starts_with("@@") {
            if let Some((old_start, new_start)) = parse_hunk_header(raw_line) {
                old_line = Some(old_start);
                new_line = Some(new_start);
            }
            continue;
        }

        let (kind, text, number) = match raw_line.chars().next() {
            Some('+') => {
                let number = new_line;
                new_line = new_line.map(|line| line + 1);
                (CodeLineKind::Added, &raw_line[1..], number)
            }
            Some('-') => {
                let number = old_line;
                old_line = old_line.map(|line| line + 1);
                (CodeLineKind::Removed, &raw_line[1..], number)
            }
            Some(' ') => {
                let number = new_line.or(old_line);
                old_line = old_line.map(|line| line + 1);
                new_line = new_line.map(|line| line + 1);
                (CodeLineKind::Context, &raw_line[1..], number)
            }
            _ => {
                let number = new_line.or(old_line);
                old_line = old_line.map(|line| line + 1);
                new_line = new_line.map(|line| line + 1);
                (CodeLineKind::Context, raw_line, number)
            }
        };

        lines.push(CodeContextLine {
            number,
            kind,
            text: text.to_owned(),
        });
    }

    lines
}

fn parse_hunk_header(header: &str) -> Option<(u64, u64)> {
    let mut parts = header.split_whitespace();
    parts.next()?;
    let old_part = parts.next()?;
    let new_part = parts.next()?;
    Some((parse_hunk_start(old_part)?, parse_hunk_start(new_part)?))
}

fn parse_hunk_start(part: &str) -> Option<u64> {
    part.trim_start_matches(['-', '+'])
        .split_once(',')
        .map_or(part.trim_start_matches(['-', '+']), |(start, _)| start)
        .parse()
        .ok()
}

fn summarize_checks(checks: &[CheckRun]) -> Option<String> {
    if checks.is_empty() {
        return None;
    }

    let mut has_failure = false;
    let mut has_pending = false;

    for check in checks {
        let value = check
            .conclusion
            .as_deref()
            .or(check.state.as_deref())
            .or(check.status.as_deref())
            .unwrap_or_default()
            .to_ascii_uppercase();

        match value.as_str() {
            "FAILURE" | "FAILED" | "ERROR" | "ACTION_REQUIRED" | "CANCELLED" | "TIMED_OUT" => {
                has_failure = true;
            }
            "SUCCESS" | "COMPLETED" | "SKIPPED" | "NEUTRAL" => {}
            _ => has_pending = true,
        }
    }

    if has_failure {
        Some("failing".to_owned())
    } else if has_pending {
        Some("pending".to_owned())
    } else {
        Some("passing".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_checks() {
        assert_eq!(summarize_checks(&[]), None);
        assert_eq!(
            summarize_checks(&[CheckRun {
                conclusion: Some("SUCCESS".to_owned()),
                state: None,
                status: None,
            }]),
            Some("passing".to_owned())
        );
        assert_eq!(
            summarize_checks(&[CheckRun {
                conclusion: Some("FAILURE".to_owned()),
                state: None,
                status: None,
            }]),
            Some("failing".to_owned())
        );
        assert_eq!(
            summarize_checks(&[CheckRun {
                conclusion: None,
                state: Some("IN_PROGRESS".to_owned()),
                status: None,
            }]),
            Some("pending".to_owned())
        );
        assert_eq!(
            summarize_checks(&[
                CheckRun {
                    conclusion: Some("SUCCESS".to_owned()),
                    state: None,
                    status: None,
                },
                CheckRun {
                    conclusion: Some("TIMED_OUT".to_owned()),
                    state: None,
                    status: None,
                }
            ]),
            Some("failing".to_owned())
        );
    }

    #[test]
    fn parses_diff_hunks() {
        let lines = parse_diff_hunk(
            "@@ -10,3 +10,4 @@ fn example() {\n context\n-removed\n+added\n unchanged",
        );

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].number, Some(10));
        assert_eq!(lines[0].kind, CodeLineKind::Context);
        assert_eq!(lines[1].number, Some(11));
        assert_eq!(lines[1].kind, CodeLineKind::Removed);
        assert_eq!(lines[2].number, Some(11));
        assert_eq!(lines[2].kind, CodeLineKind::Added);
        assert_eq!(lines[3].number, Some(12));
        assert_eq!(lines[3].text, "unchanged");
    }

    #[test]
    fn converts_search_pull_requests_to_domain_model() {
        let pr = PullRequest::from(SearchPullRequest {
            repository: Repository {
                name_with_owner: "owner/repo".to_owned(),
            },
            number: 12,
            title: "Add feature".to_owned(),
            author: Some(User {
                login: "alice".to_owned(),
            }),
            head_ref_name: "feature".to_owned(),
            review_decision: Some("APPROVED".to_owned()),
            status_check_rollup: Some(vec![CheckRun {
                conclusion: Some("SUCCESS".to_owned()),
                state: None,
                status: None,
            }]),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
            state: "OPEN".to_owned(),
            is_draft: false,
            url: "https://example.test/pr".to_owned(),
        });

        assert_eq!(pr.repo, "owner/repo");
        assert_eq!(pr.author, "alice");
        assert_eq!(pr.head_ref, "feature");
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.check_status.as_deref(), Some("passing"));
    }

    #[test]
    fn converts_dashboard_search_response_to_domain_model() {
        let response: DashboardSearchResponse = serde_json::from_str(
            r#"
            {
              "data": {
                "search": {
                  "nodes": [{
                    "repository": { "nameWithOwner": "owner/repo" },
                    "number": 12,
                    "title": "Add feature",
                    "author": { "login": "carol" },
                    "headRefName": "feature/add",
                    "reviewDecision": "REVIEW_REQUIRED",
                    "updatedAt": "2026-07-01T10:00:00Z",
                    "isDraft": false,
                    "url": "https://example.test/pr",
                    "reviews": {
                      "nodes": [
                        {
                          "author": { "login": "bob", "__typename": "User" },
                          "state": "APPROVED"
                        },
                        {
                          "author": { "login": "dependabot", "__typename": "Bot" },
                          "state": "COMMENTED"
                        }
                      ]
                    },
                    "reviewRequests": {
                      "nodes": [
                        {
                          "requestedReviewer": { "login": "alice", "__typename": "User" }
                        },
                        {
                          "requestedReviewer": { "name": "platform", "__typename": "Team" }
                        }
                      ]
                    },
                    "commits": {
                      "nodes": [{
                        "commit": { "statusCheckRollup": { "state": "SUCCESS" } }
                      }]
                    }
                  }]
                }
              }
            }
            "#,
        )
        .unwrap();

        let (prs, cursor) = response.into_page();

        assert_eq!(prs.len(), 1);
        assert_eq!(cursor, None);
        let pr = &prs[0];
        assert_eq!(pr.repo, "owner/repo");
        assert_eq!(pr.author, "carol");
        assert_eq!(pr.head_ref, "feature/add");
        assert_eq!(pr.review_decision.as_deref(), Some("REVIEW_REQUIRED"));
        assert_eq!(pr.check_status.as_deref(), Some("passing"));
        assert_eq!(pr.review_requested, vec!["alice".to_owned()]);
        assert_eq!(
            pr.reviewers,
            vec![
                Reviewer {
                    login: "alice".to_owned(),
                    state: ReviewerState::Requested,
                },
                Reviewer {
                    login: "bob".to_owned(),
                    state: ReviewerState::Approved,
                },
                Reviewer {
                    login: "platform".to_owned(),
                    state: ReviewerState::Requested,
                }
            ]
        );
    }

    #[test]
    fn converts_combined_dashboard_response_to_sections() {
        let response: DashboardResponse = serde_json::from_str(
            r#"
            {
              "data": {
                "myPrs": {
                  "nodes": [{
                    "repository": { "nameWithOwner": "owner/mine" },
                    "number": 1,
                    "title": "Mine",
                    "author": { "login": "octocat" },
                    "headRefName": "mine-branch",
                    "reviewDecision": null,
                    "updatedAt": "2026-07-01T10:00:00Z",
                    "isDraft": false,
                    "url": "https://example.test/mine",
                    "reviews": { "nodes": [] },
                    "reviewRequests": { "nodes": [] },
                    "commits": { "nodes": [] }
                  }]
                },
                "reviewRequests": {
                  "nodes": [
                    {
                      "repository": { "nameWithOwner": "owner/review" },
                      "number": 2,
                      "title": "Review me",
                      "author": { "login": "alice" },
                      "headRefName": "review-branch",
                      "reviewDecision": "REVIEW_REQUIRED",
                      "updatedAt": "2026-07-01T11:00:00Z",
                      "isDraft": false,
                      "url": "https://example.test/review",
                      "reviews": { "nodes": [] },
                      "reviewRequests": {
                        "nodes": [{
                          "requestedReviewer": { "login": "octocat", "__typename": "User" }
                        }]
                      },
                      "commits": { "nodes": [] }
                    },
                    {
                      "repository": { "nameWithOwner": "owner/team" },
                      "number": 3,
                      "title": "Team review",
                      "author": { "login": "bob" },
                      "headRefName": "team-branch",
                      "reviewDecision": "REVIEW_REQUIRED",
                      "updatedAt": "2026-07-01T12:00:00Z",
                      "isDraft": false,
                      "url": "https://example.test/team",
                      "reviews": { "nodes": [] },
                      "reviewRequests": {
                        "nodes": [{
                          "requestedReviewer": { "name": "platform", "__typename": "Team" }
                        }]
                      },
                      "commits": { "nodes": [] }
                    }
                  ]
                }
              }
            }
            "#,
        )
        .unwrap();

        let ((my_prs, my_cursor), (reviews, review_cursor)) = response.into_page("octocat");

        assert_eq!(my_prs.len(), 1);
        assert_eq!(my_cursor, None);
        assert_eq!(my_prs[0].repo, "owner/mine");
        assert_eq!(my_prs[0].head_ref, "mine-branch");
        assert_eq!(reviews.len(), 1);
        assert_eq!(review_cursor, None);
        assert_eq!(reviews[0].repo, "owner/review");
        assert_eq!(reviews[0].head_ref, "review-branch");
        assert_eq!(reviews[0].review_requested, vec!["octocat".to_owned()]);
        assert_eq!(reviews[0].reviewers[0].login, "octocat");
    }

    #[test]
    fn converts_detail_pull_requests_to_domain_model() {
        let detail = DetailPullRequest {
            number: 5,
            title: "Fix bug".to_owned(),
            author: None,
            review_decision: None,
            status_check_rollup: None,
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
            is_draft: false,
            url: "https://example.test/pr".to_owned(),
            body: "body".to_owned(),
            state: "OPEN".to_owned(),
            mergeable: Some("MERGEABLE".to_owned()),
            head_ref_name: "feature".to_owned(),
            base_ref_name: "main".to_owned(),
            comments: vec![GhComment {
                author: Some(User {
                    login: "bob".to_owned(),
                }),
                body: "Looks good".to_owned(),
                created_at: "2026-07-01T11:00:00Z".to_owned(),
                url: "https://example.test/comment".to_owned(),
            }],
            reviews: vec![GhReview {
                author: Some(User {
                    login: "carol".to_owned(),
                }),
                state: "APPROVED".to_owned(),
                body: "approved".to_owned(),
                submitted_at: "2026-07-01T12:00:00Z".to_owned(),
            }],
        }
        .into_detail("owner/repo".to_owned());

        assert_eq!(detail.pr.repo, "owner/repo");
        assert_eq!(detail.pr.author, "");
        assert_eq!(detail.pr.head_ref, "feature");
        assert_eq!(detail.body, "body");
        assert_eq!(detail.discussion.len(), 1);
        assert_eq!(detail.discussion[0].author, "bob");
        assert_eq!(detail.reviews.len(), 1);
        assert_eq!(detail.reviews[0].author, "carol");
    }

    #[test]
    fn skips_review_threads_without_comments() {
        let thread = ReviewThreadNode {
            is_resolved: true,
            path: "src/main.rs".to_owned(),
            line: Some(1),
            original_line: None,
            comments: ReviewThreadCommentConnection { nodes: Vec::new() },
        };

        assert!(thread.into_discussion_item().is_none());
    }

    #[test]
    fn maps_review_threads_to_discussion_items() {
        let response: ReviewThreadsResponse = serde_json::from_str(
            r#"
            {
              "data": {
                "repository": {
                  "pullRequest": {
                    "reviewThreads": {
                      "nodes": [{
                        "isResolved": false,
                        "path": "src/main.rs",
                        "line": 42,
                        "originalLine": null,
                        "comments": {
                          "nodes": [
                            {
                              "author": { "login": "alice" },
                              "body": "Could we change this?",
                              "diffHunk": "@@ -41,2 +41,2 @@\n old\n+new",
                              "createdAt": "2026-07-01T10:00:00Z",
                              "url": "https://example.test/thread#1"
                            },
                            {
                              "author": { "login": "nikita" },
                              "body": "Yep.",
                              "diffHunk": "@@ -41,2 +41,2 @@\n old\n+new",
                              "createdAt": "2026-07-01T10:05:00Z",
                              "url": "https://example.test/thread#2"
                            }
                          ]
                        }
                      }]
                    }
                  }
                }
              }
            }
            "#,
        )
        .unwrap();

        let (items, cursor) = response.into_page();
        assert_eq!(items.len(), 1);
        assert_eq!(cursor, None);
        assert_eq!(items[0].author, "alice");
        assert_eq!(items[0].replies.len(), 1);
        assert_eq!(items[0].replies[0].author, "nikita");
        assert!(matches!(
            items[0].kind,
            DiscussionKind::ReviewThread { resolved: false }
        ));
        let context = items[0].code_context.as_ref().unwrap();
        assert_eq!(context.path, "src/main.rs");
        assert_eq!(context.highlighted_line, Some(42));
        assert!(context.lines.iter().any(|line| line.text == "new"));
    }

    #[test]
    fn exposes_page_cursors_when_more_results_exist() {
        let response: DashboardSearchResponse = serde_json::from_str(
            r#"{"data":{"search":{"nodes":[],"pageInfo":{"hasNextPage":true,"endCursor":"next-page"}}}}"#,
        )
        .unwrap();
        let (prs, cursor) = response.into_page();
        assert!(prs.is_empty());
        assert_eq!(cursor.as_deref(), Some("next-page"));

        let response: ReviewThreadsResponse = serde_json::from_str(
            r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[],"pageInfo":{"hasNextPage":true,"endCursor":"next-thread"}}}}}}"#,
        )
        .unwrap();
        let (items, cursor) = response.into_page();
        assert!(items.is_empty());
        assert_eq!(cursor.as_deref(), Some("next-thread"));
    }

    #[test]
    fn parses_hunks_without_header_as_context_without_numbers() {
        let lines = parse_diff_hunk("plain line\n+added");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].number, None);
        assert_eq!(lines[0].kind, CodeLineKind::Context);
        assert_eq!(lines[1].number, None);
        assert_eq!(lines[1].kind, CodeLineKind::Added);
    }
}
