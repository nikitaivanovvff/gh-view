use super::{GhStatus, PullRequestSource};
use crate::model::{
    CodeContext, CodeContextLine, CodeLineKind, DiscussionItem, DiscussionKind, DiscussionReply,
    PrReview, PullRequest, PullRequestDetail,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::process::Command;

const SEARCH_FIELDS: &str = "repository,number,title,author,updatedAt,state,isDraft,url";
const DETAIL_FIELDS: &str = "number,title,author,updatedAt,isDraft,url,body,state,mergeable,headRefName,baseRefName,reviewDecision,statusCheckRollup,comments,reviews";
const REVIEW_THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 50) {
        nodes {
          isResolved
          path
          line
          originalLine
          comments(first: 50) {
            nodes {
              author { login }
              body
              diffHunk
              createdAt
              url
            }
          }
        }
      }
    }
  }
}
"#;

#[derive(Clone)]
pub struct GhClient;

impl GhClient {
    pub fn new() -> Self {
        Self
    }

    fn current_user_login(&self) -> Result<String> {
        let output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .output()
            .context("failed to run gh api user")?;
        if !output.status.success() {
            bail!(command_error(&output));
        }

        let login = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if login.is_empty() {
            bail!("gh api user returned an empty login");
        }

        Ok(login)
    }

    fn search_prs(&self, filter: &[&str]) -> Result<Vec<PullRequest>> {
        let mut args = vec!["search", "prs", "--state", "open", "--json", SEARCH_FIELDS];
        args.extend_from_slice(filter);

        let output = Command::new("gh")
            .args(args)
            .output()
            .context("failed to run gh search prs")?;

        if !output.status.success() {
            bail!(command_error(&output));
        }

        let rows: Vec<SearchPullRequest> = serde_json::from_slice(&output.stdout)
            .context("failed to parse gh search prs JSON output")?;
        Ok(rows.into_iter().map(PullRequest::from).collect())
    }

    fn pr_view(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
        let number = pr.number.to_string();
        let output = Command::new("gh")
            .args([
                "pr",
                "view",
                &number,
                "--repo",
                &pr.repo,
                "--json",
                DETAIL_FIELDS,
            ])
            .output()
            .context("failed to run gh pr view")?;

        if !output.status.success() {
            bail!(command_error(&output));
        }

        let row: DetailPullRequest = serde_json::from_slice(&output.stdout)
            .context("failed to parse gh pr view JSON output")?;
        Ok(row.into_detail(pr.repo.clone()))
    }

    fn review_threads(&self, pr: &PullRequest) -> Result<Vec<DiscussionItem>> {
        let (owner, name) = split_repo(&pr.repo)?;
        let number = pr.number.to_string();
        let query_field = format!("query={REVIEW_THREADS_QUERY}");
        let owner_field = format!("owner={owner}");
        let name_field = format!("name={name}");
        let number_field = format!("number={number}");

        let output = Command::new("gh")
            .args([
                "api",
                "graphql",
                "-f",
                &query_field,
                "-F",
                &owner_field,
                "-F",
                &name_field,
                "-F",
                &number_field,
            ])
            .output()
            .context("failed to run gh api graphql for review threads")?;

        if !output.status.success() {
            bail!(command_error(&output));
        }

        let response: ReviewThreadsResponse = serde_json::from_slice(&output.stdout)
            .context("failed to parse gh review threads GraphQL output")?;
        let items = response.into_discussion_items();
        Ok(items)
    }
}

impl PullRequestSource for GhClient {
    fn clone_box(&self) -> Box<dyn PullRequestSource> {
        Box::new(self.clone())
    }

    fn status(&self) -> GhStatus {
        let Some(version) = gh_version() else {
            return GhStatus::Missing;
        };

        let auth_status = Command::new("gh").args(["auth", "status"]).output();
        match auth_status {
            Ok(output) if output.status.success() => GhStatus::Ready { version },
            Ok(output) => GhStatus::Unauthenticated {
                message: command_error(&output),
            },
            Err(error) => GhStatus::Unauthenticated {
                message: error.to_string(),
            },
        }
    }

    fn current_user(&self) -> Result<String> {
        self.current_user_login()
    }

    fn fetch_my_prs(&self) -> Result<Vec<PullRequest>> {
        self.search_prs(&["--author", "@me"])
    }

    fn fetch_review_requests(&self, login: &str) -> Result<Vec<PullRequest>> {
        self.search_prs(&["--review-requested", login])
    }

    fn fetch_pr_detail(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
        self.pr_view(pr)
    }

    fn fetch_pr_discussion(&self, pr: &PullRequest) -> Result<Vec<DiscussionItem>> {
        let mut discussion = self.review_threads(pr)?;
        discussion.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.url.cmp(&right.url))
        });
        Ok(discussion)
    }
}

pub fn gh_version() -> Option<String> {
    let output = Command::new("gh").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(str::trim).map(str::to_owned)
}

fn command_error(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("command exited with {}", output.status)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPullRequest {
    repository: Repository,
    number: u64,
    title: String,
    author: Option<User>,
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
struct DetailPullRequest {
    number: u64,
    title: String,
    author: Option<User>,
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
    head_ref_name: String,
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
struct ReviewThreadsResponse {
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
            url: value.url,
            updated_at: value.updated_at,
            state: value.state,
            is_draft: value.is_draft,
            review_decision: value.review_decision,
            check_status: summarize_checks(value.status_check_rollup.unwrap_or_default()),
            reviewers: Vec::new(),
        }
    }
}

impl ReviewThreadsResponse {
    fn into_discussion_items(self) -> Vec<DiscussionItem> {
        self.data
            .repository
            .pull_request
            .review_threads
            .nodes
            .into_iter()
            .filter_map(ReviewThreadNode::into_discussion_item)
            .collect()
    }
}

impl ReviewThreadNode {
    fn into_discussion_item(self) -> Option<DiscussionItem> {
        let mut comments = self.comments.nodes.into_iter();
        let first = comments.next()?;
        let highlighted_line = self.line.or(self.original_line);
        let lines = parse_diff_hunk(&first.diff_hunk);
        let start_line = lines.iter().find_map(|line| line.number);
        let code_context = if self.path.is_empty() && lines.is_empty() {
            None
        } else {
            Some(CodeContext {
                path: self.path,
                start_line,
                highlighted_line,
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
    fn into_detail(self, repo: String) -> PullRequestDetail {
        let pr = PullRequest {
            repo,
            number: self.number,
            title: self.title,
            author: self.author.map(|author| author.login).unwrap_or_default(),
            url: self.url,
            updated_at: self.updated_at,
            state: self.state.clone(),
            is_draft: self.is_draft,
            review_decision: self.review_decision,
            check_status: summarize_checks(self.status_check_rollup.unwrap_or_default()),
            reviewers: Vec::new(),
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

fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(owner, name)| !owner.is_empty() && !name.is_empty())
        .context("repository name must be in owner/name format")
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

fn summarize_checks(checks: Vec<CheckRun>) -> Option<String> {
    if checks.is_empty() {
        return None;
    }

    let mut has_failure = false;
    let mut has_pending = false;

    for check in checks {
        let value = check
            .conclusion
            .or(check.state)
            .or(check.status)
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
        assert_eq!(summarize_checks(Vec::new()), None);
        assert_eq!(
            summarize_checks(vec![CheckRun {
                conclusion: Some("SUCCESS".to_owned()),
                state: None,
                status: None,
            }]),
            Some("passing".to_owned())
        );
        assert_eq!(
            summarize_checks(vec![CheckRun {
                conclusion: Some("FAILURE".to_owned()),
                state: None,
                status: None,
            }]),
            Some("failing".to_owned())
        );
        assert_eq!(
            summarize_checks(vec![CheckRun {
                conclusion: None,
                state: Some("IN_PROGRESS".to_owned()),
                status: None,
            }]),
            Some("pending".to_owned())
        );
        assert_eq!(
            summarize_checks(vec![
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
    fn splits_repo_names() {
        assert_eq!(split_repo("owner/name").unwrap(), ("owner", "name"));
        assert!(split_repo("owner").is_err());
        assert!(split_repo("owner/").is_err());
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
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.check_status.as_deref(), Some("passing"));
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

        let items = response.into_discussion_items();
        assert_eq!(items.len(), 1);
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
