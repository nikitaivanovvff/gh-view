mod types;

use self::types::{
    DashboardResponse, DashboardSearchResponse, DetailPullRequest, ReviewThreadsResponse,
    SearchPullRequest,
};
use super::{GhStatus, PullRequestSource};
use crate::model::{DiscussionItem, PullRequest, PullRequestDetail};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
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

    fn search_dashboard_graphql(&self, query: &str) -> Result<Vec<PullRequest>> {
        let graphql = dashboard_search_query(query);
        let response: DashboardSearchResponse = self.graphql(graphql)?;
        Ok(response.into_pull_requests())
    }

    fn dashboard_graphql(&self, login: &str) -> Result<(Vec<PullRequest>, Vec<PullRequest>)> {
        let graphql = dashboard_query(login);
        let response: DashboardResponse = self.graphql(graphql)?;
        Ok(response.into_dashboard(login))
    }

    fn graphql<T: DeserializeOwned>(&self, graphql: String) -> Result<T> {
        let query_field = format!("query={graphql}");
        let output = Command::new("gh")
            .args(["api", "graphql", "-f", &query_field])
            .output()
            .context("failed to run gh api graphql")?;

        if !output.status.success() {
            bail!(command_error(&output));
        }

        serde_json::from_slice(&output.stdout).context("failed to parse gh GraphQL output")
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

    fn fetch_my_prs(&self, login: &str) -> Result<Vec<PullRequest>> {
        self.search_dashboard_graphql(&format!("is:pr is:open author:{login} archived:false"))
            .or_else(|_| self.search_prs(&["--author", login]))
    }

    fn fetch_dashboard(&self, login: &str) -> Result<(Vec<PullRequest>, Vec<PullRequest>)> {
        self.dashboard_graphql(login).or_else(|_| {
            Ok((
                self.search_prs(&["--author", login])?,
                self.search_prs(&["--review-requested", login])?,
            ))
        })
    }

    fn fetch_review_requests(&self, login: &str) -> Result<Vec<PullRequest>> {
        self.search_dashboard_graphql(&format!(
            "is:pr is:open review-requested:{login} archived:false"
        ))
        .map(|prs| {
            prs.into_iter()
                .filter(|pr| pr.review_requested.iter().any(|reviewer| reviewer == login))
                .collect()
        })
        .or_else(|_| self.search_prs(&["--review-requested", login]))
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

fn dashboard_search_query(query: &str) -> String {
    let escaped_query = query.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{
  search(query: "{escaped_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

fn dashboard_query(login: &str) -> String {
    let my_query = escape_graphql_string(&format!("is:pr is:open author:{login} archived:false"));
    let review_query = escape_graphql_string(&format!(
        "is:pr is:open review-requested:{login} archived:false"
    ));
    format!(
        r#"{{
  myPrs: search(query: "{my_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
  reviewRequests: search(query: "{review_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

fn escape_graphql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

const DASHBOARD_PULL_REQUEST_FRAGMENT: &str = r#"
fragment DashboardPullRequestFields on PullRequest {
  repository { nameWithOwner }
  number
  title
  url
  isDraft
  reviewDecision
  updatedAt
  author { login }
  reviews(last: 20) {
    nodes {
      author { login __typename }
      state
    }
  }
  reviewRequests(first: 20) {
    nodes {
      requestedReviewer {
        ... on User { login __typename }
        ... on Team { name __typename }
      }
    }
  }
  commits(last: 1) {
    nodes {
      commit {
        statusCheckRollup { state }
      }
    }
  }
}
"#;

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

fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(owner, name)| !owner.is_empty() && !name.is_empty())
        .context("repository name must be in owner/name format")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    #[test]
    fn graphql_string_escaping_handles_quotes_and_backslashes() {
        assert_eq!(escape_graphql_string(r#"owner\"repo"#), r#"owner\\\"repo"#);

        let query = dashboard_search_query(r#"author:octo\"cat"#);
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains("DashboardPullRequestFields"));
    }

    #[test]
    fn dashboard_query_builds_both_dashboard_sections() {
        let query = dashboard_query(r#"octo\"cat"#);

        assert!(query.contains("myPrs: search"));
        assert!(query.contains("reviewRequests: search"));
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains(r#"review-requested:octo\\\"cat"#));
    }

    #[test]
    fn command_error_prefers_stderr_then_stdout_then_status() {
        assert_eq!(command_error(&output(b"details", b"ignored")), "details");
        assert_eq!(
            command_error(&output(b"", b"stdout details")),
            "stdout details"
        );
        assert!(command_error(&output(b"", b"")).contains("command exited with"));
    }

    #[test]
    fn splits_repo_names() {
        assert_eq!(split_repo("owner/name").unwrap(), ("owner", "name"));
        assert!(split_repo("owner").is_err());
        assert!(split_repo("owner/").is_err());
    }

    fn output(stderr: &[u8], stdout: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(1),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
}
