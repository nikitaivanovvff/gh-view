#[cfg(test)]
use super::DEFAULT_GH_COMMAND_TIMEOUT_SECONDS;
use super::{GhError, GhStatus, MockErrorMode, PullRequestSource};
use crate::model::{
    CheckStatus, CodeContext, CodeContextLine, CodeLineKind, DiscussionItem, DiscussionKind,
    DiscussionReply, PrReview, PullRequest, PullRequestDetail, Reviewer, ReviewerState,
};
use anyhow::{Result, bail};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockGhClient {
    error_mode: Arc<Mutex<Option<MockErrorMode>>>,
    gh_timeout_seconds: u64,
}

impl MockGhClient {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_GH_COMMAND_TIMEOUT_SECONDS)
    }

    pub fn with_timeout(gh_timeout_seconds: u64) -> Self {
        Self {
            error_mode: Arc::new(Mutex::new(None)),
            gh_timeout_seconds,
        }
    }

    fn maybe_fail(&self) -> Result<()> {
        match self.mock_error_mode() {
            Some(MockErrorMode::GitHubDown) => bail!(GhError::GitHubOutage(
                "HTTP 503 Service Unavailable: GitHub API is currently unavailable".to_owned()
            )),
            Some(MockErrorMode::Timeout) => bail!(GhError::Timeout(format!(
                "gh command timed out after {}s: gh api graphql",
                self.gh_timeout_seconds
            ))),
            Some(MockErrorMode::Generic) => bail!(GhError::Command(
                "mock gh command failed with a generic API error".to_owned()
            )),
            Some(MockErrorMode::Auth) => bail!(GhError::Unauthenticated(
                "mock gh requires authentication; run `gh auth login`".to_owned()
            )),
            None => Ok(()),
        }
    }
}

impl PullRequestSource for MockGhClient {
    fn clone_box(&self) -> Box<dyn PullRequestSource> {
        Box::new(self.clone())
    }

    fn status(&self) -> GhStatus {
        GhStatus::Ready {
            version: "mock-gh 0.0.0".to_owned(),
        }
    }

    fn is_mock(&self) -> bool {
        true
    }

    fn mock_error_mode(&self) -> Option<MockErrorMode> {
        self.error_mode.lock().ok().and_then(|mode| *mode)
    }

    fn set_mock_error_mode(&mut self, mode: Option<MockErrorMode>) {
        if let Ok(mut current) = self.error_mode.lock() {
            *current = mode;
        }
    }

    fn current_user(&self) -> Result<String> {
        if self.mock_error_mode() == Some(MockErrorMode::Auth) {
            self.maybe_fail()?;
        }
        Ok("nikita".to_owned())
    }

    fn fetch_my_prs(&self, _login: &str) -> Result<Vec<PullRequest>> {
        self.maybe_fail()?;
        Ok(vec![
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 37,
                title: "Untangle dashboard row projection for large repos",
                head_ref: "dashboard-row-projection",
                author: "nikita",
                updated_at: "2026-05-28T09:00:00Z",
                is_draft: false,
                review_decision: Some("CHANGES_REQUESTED"),
                check_status: Some("failing"),
                reviewers: vec!["alice"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 38,
                title: "Add repo pagination state to dashboard",
                head_ref: "repo-pagination-state",
                author: "nikita",
                updated_at: "2026-06-07T11:15:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("pending"),
                reviewers: vec!["bob"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 39,
                title: "Polish stale PR age labels",
                head_ref: "idle-duration-labels",
                author: "nikita",
                updated_at: "2026-06-18T14:20:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("passing"),
                reviewers: vec!["carol"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 40,
                title: "Support mouse hit testing for PR rows",
                head_ref: "mouse-hit-testing",
                author: "nikita",
                updated_at: "2026-06-24T18:30:00Z",
                is_draft: false,
                review_decision: None,
                check_status: Some("pending"),
                reviewers: vec!["core-team"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 41,
                title: "Render compact reviewer summaries",
                head_ref: "reviewer-summary",
                author: "nikita",
                updated_at: "2026-06-30T15:45:00Z",
                is_draft: true,
                review_decision: None,
                check_status: Some("pending"),
                reviewers: vec!["alice", "bob"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 42,
                title: "Build dashboard MVP",
                head_ref: "dashboard-mvp",
                author: "nikita",
                updated_at: "2026-07-05T10:00:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("passing"),
                reviewers: vec!["alice", "bob"],
            }),
            mock_pr(MockPr {
                repo: "earendil/gh-view",
                number: 43,
                title: "Wire PR detail route",
                head_ref: "phase-2-detail",
                author: "nikita",
                updated_at: "2026-07-06T18:30:00Z",
                is_draft: false,
                review_decision: Some("APPROVED"),
                check_status: Some("passing"),
                reviewers: vec!["dave"],
            }),
            mock_pr(MockPr {
                repo: "earendil/pi",
                number: 108,
                title: "Fix terminal restore on panic",
                head_ref: "panic-restore",
                author: "nikita",
                updated_at: "2026-07-05T09:15:00Z",
                is_draft: false,
                review_decision: Some("APPROVED"),
                check_status: Some("failing"),
                reviewers: vec!["carol"],
            }),
        ])
    }

    fn fetch_review_requests(&self, _login: &str) -> Result<Vec<PullRequest>> {
        self.maybe_fail()?;
        Ok(vec![
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 4,
                title: "Document overseer recovery flow",
                head_ref: "recovery-docs",
                author: "alice",
                updated_at: "2026-06-12T08:45:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("passing"),
                reviewers: vec!["nikita"],
            }),
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 5,
                title: "Split worker supervision logs",
                head_ref: "worker-log-split",
                author: "dave",
                updated_at: "2026-06-19T10:30:00Z",
                is_draft: false,
                review_decision: None,
                check_status: Some("pending"),
                reviewers: vec!["nikita"],
            }),
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 6,
                title: "Surface stuck agent heartbeats",
                head_ref: "agent-heartbeats",
                author: "carol",
                updated_at: "2026-06-23T09:45:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("passing"),
                reviewers: vec!["nikita", "alice"],
            }),
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 7,
                title: "Add agent registry persistence",
                head_ref: "agent-registry",
                author: "alice",
                updated_at: "2026-06-27T08:45:00Z",
                is_draft: false,
                review_decision: Some("REVIEW_REQUIRED"),
                check_status: Some("passing"),
                reviewers: vec!["nikita"],
            }),
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 8,
                title: "Add compact task queue metrics",
                head_ref: "queue-metrics",
                author: "erin",
                updated_at: "2026-07-01T12:00:00Z",
                is_draft: true,
                review_decision: None,
                check_status: Some("pending"),
                reviewers: vec!["nikita"],
            }),
            mock_pr(MockPr {
                repo: "earendil/overseer",
                number: 9,
                title: "Refactor IPC protocol errors",
                head_ref: "ipc-errors",
                author: "bob",
                updated_at: "2026-07-06T12:00:00Z",
                is_draft: false,
                review_decision: Some("CHANGES_REQUESTED"),
                check_status: Some("failing"),
                reviewers: vec!["nikita", "dave"],
            }),
            mock_pr(MockPr {
                repo: "acme/widgets",
                number: 314,
                title: "Document widget release flow",
                head_ref: "widget-release-docs",
                author: "carol",
                updated_at: "2026-07-04T16:20:00Z",
                is_draft: false,
                review_decision: None,
                check_status: None,
                reviewers: vec![],
            }),
        ])
    }

    fn fetch_pr_detail(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
        self.maybe_fail()?;
        Ok(PullRequestDetail {
            pr: pr.clone(),
            body: "This PR wires the phase-2 detail screen. It keeps GitHub access behind the data-source trait and renders comments/reviews in a flat terminal layout.".to_owned(),
            state: "OPEN".to_owned(),
            mergeable: Some("MERGEABLE".to_owned()),
            head_ref: "phase-2-detail".to_owned(),
            base_ref: "main".to_owned(),
            discussion: mock_discussion(pr),
            reviews: vec![PrReview {
                author: "bob".to_owned(),
                state: "COMMENTED".to_owned(),
                body: "Structure looks good for the MVP.".to_owned(),
                submitted_at: "2026-06-30T11:30:00Z".to_owned(),
            }],
        })
    }
}

struct MockPr<'a> {
    repo: &'a str,
    number: u64,
    title: &'a str,
    head_ref: &'a str,
    author: &'a str,
    updated_at: &'a str,
    is_draft: bool,
    review_decision: Option<&'a str>,
    check_status: Option<&'a str>,
    reviewers: Vec<&'a str>,
}

fn mock_discussion(pr: &PullRequest) -> Vec<DiscussionItem> {
    vec![
        DiscussionItem {
            kind: DiscussionKind::ReviewThread { resolved: false },
            author: "alice".to_owned(),
            body: "This is the area I was talking about: the comments pane has a lot of unused horizontal space. Could we show the code context next to the thread instead?".to_owned(),
            created_at: "2026-06-30T11:00:00Z".to_owned(),
            url: format!("{}#discussion_r1", pr.url),
            replies: vec![DiscussionReply {
                author: "nikita".to_owned(),
                body: "Agreed. I normalized comments and review threads into one carousel and added an optional code pane.".to_owned(),
                created_at: "2026-06-30T11:20:00Z".to_owned(),
            }],
            code_context: Some(CodeContext {
                path: "src/ui/mod.rs".to_owned(),
                start_line: Some(138),
                highlighted_line: Some(146),
                highlighted_kind: None,
                lines: vec![
                    code_line(138, CodeLineKind::Context, "lines.push(Line::styled(\"COMMENTS\", theme::muted()));"),
                    code_line(139, CodeLineKind::Context, "if detail.comments.is_empty() {"),
                    code_line(140, CodeLineKind::Context, "    lines.push(Line::styled(\"  none\", theme::muted()));"),
                    code_line(141, CodeLineKind::Context, "} else {"),
                    code_line(142, CodeLineKind::Removed, "    for comment in &detail.comments {"),
                    code_line(143, CodeLineKind::Removed, "        push_wrapped(&mut lines, &comment.body, width, \"    \", theme::normal());"),
                    code_line(144, CodeLineKind::Added, "    let item = selected_discussion(detail, app.discussion_selected);"),
                    code_line(145, CodeLineKind::Added, "    render_discussion_panes(frame, chunks[1], item);"),
                    code_line(146, CodeLineKind::Context, "}"),
                ],
            }),
        },
        DiscussionItem {
            kind: DiscussionKind::IssueComment,
            author: "carol".to_owned(),
            body: "General question: should issue comments without line context still appear in the same carousel? I think yes, with an empty code pane.".to_owned(),
            created_at: "2026-06-30T11:45:00Z".to_owned(),
            url: format!("{}#issuecomment-3", pr.url),
            replies: Vec::new(),
            code_context: None,
        },
    ]
}

fn code_line(number: u64, kind: CodeLineKind, text: &str) -> CodeContextLine {
    CodeContextLine {
        number: Some(number),
        kind,
        text: text.to_owned(),
    }
}

fn reviewer_state(review_decision: Option<&str>, login: &str) -> ReviewerState {
    match review_decision {
        Some("APPROVED") => ReviewerState::Approved,
        Some("CHANGES_REQUESTED") if login == "nikita" => ReviewerState::ChangesRequested,
        _ => ReviewerState::Requested,
    }
}

fn mock_pr(input: MockPr<'_>) -> PullRequest {
    PullRequest {
        repo: input.repo.to_owned(),
        number: input.number,
        title: input.title.to_owned(),
        author: input.author.to_owned(),
        head_ref: input.head_ref.to_owned(),
        url: format!("https://github.com/{}/pull/{}", input.repo, input.number),
        updated_at: input.updated_at.to_owned(),
        state: "OPEN".to_owned(),
        is_draft: input.is_draft,
        review_decision: input.review_decision.map(str::to_owned),
        check_status: input.check_status.map(CheckStatus::from_label),
        reviewers: input
            .reviewers
            .iter()
            .copied()
            .map(|login| Reviewer {
                login: login.to_owned(),
                state: reviewer_state(input.review_decision, login),
            })
            .collect(),
        review_requested: input.reviewers.into_iter().map(str::to_owned).collect(),
    }
}
