#[path = "gh/types.rs"]
mod types;

use self::types::{
    DashboardResponse, DashboardSearchResponse, DetailPullRequest, ReviewThreadsResponse,
    SearchPullRequest,
};
use super::{GhError, GhStatus, PullRequestSource};
use crate::github::command::{GhCommand, command_error};
use crate::model::{DiscussionItem, PullRequest, PullRequestDetail};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use std::time::Duration;

const SEARCH_FIELDS: &str = "repository,number,title,author,updatedAt,state,isDraft,url";
const DETAIL_FIELDS: &str = "number,title,author,updatedAt,isDraft,url,body,state,mergeable,headRefName,baseRefName,reviewDecision,statusCheckRollup,comments,reviews";
const REVIEW_THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      headRefOid
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
pub struct GhClient {
    command: GhCommand,
}

impl GhClient {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            command: GhCommand::new(Duration::from_secs(timeout_seconds.max(1))),
        }
    }

    fn current_user_login(&self) -> Result<String> {
        let output = self.run_gh(["api", "user", "--jq", ".login"])?;
        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
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

        let output = self.run_gh(args)?;

        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
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
        let output = self.run_gh(["api", "graphql", "-f", &query_field])?;

        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
        }

        serde_json::from_slice(&output.stdout).context("failed to parse gh GraphQL output")
    }

    fn pr_view(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
        let number = pr.number.to_string();
        let output = self.run_gh([
            "pr",
            "view",
            &number,
            "--repo",
            &pr.repo,
            "--json",
            DETAIL_FIELDS,
        ])?;

        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
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

        let output = self.run_gh([
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
        ])?;

        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
        }

        let response: ReviewThreadsResponse = serde_json::from_slice(&output.stdout)
            .context("failed to parse gh review threads GraphQL output")?;
        let head_ref_oid = response.head_ref_oid().to_owned();
        let items = response.into_discussion_items_with_context(|path, line| {
            self.file_context(owner, name, &head_ref_oid, path, line)
                .ok()
        });
        Ok(items)
    }

    fn file_context(
        &self,
        owner: &str,
        name: &str,
        ref_oid: &str,
        path: &str,
        line: u64,
    ) -> Result<Vec<crate::model::CodeContextLine>> {
        let endpoint = format!("repos/{owner}/{name}/contents/{}", encode_path(path));
        let ref_field = format!("ref={ref_oid}");
        let output = self.run_gh([
            "api",
            &endpoint,
            "--method",
            "GET",
            "-H",
            "Accept: application/vnd.github.raw",
            "-f",
            &ref_field,
        ])?;

        if !output.status.success() {
            bail!(GhError::from_command_output(command_error(&output)));
        }

        Ok(source_context_lines(
            &String::from_utf8_lossy(&output.stdout),
            line,
        ))
    }
}

impl PullRequestSource for GhClient {
    fn clone_box(&self) -> Box<dyn PullRequestSource> {
        Box::new(self.clone())
    }

    fn status(&self) -> GhStatus {
        let Some(version) = self.command.version() else {
            return GhStatus::Missing;
        };

        let auth_status = self.run_gh(["auth", "status"]);
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

impl GhClient {
    fn run_gh<I, S>(&self, args: I) -> Result<std::process::Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.command.run(args)
    }
}

fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(owner, name)| !owner.is_empty() && !name.is_empty())
        .context("repository name must be in owner/name format")
}

fn source_context_lines(text: &str, highlighted_line: u64) -> Vec<crate::model::CodeContextLine> {
    const CONTEXT_RADIUS: u64 = 10;

    let start = highlighted_line.saturating_sub(CONTEXT_RADIUS).max(1);
    let end = highlighted_line.saturating_add(CONTEXT_RADIUS);

    text.lines()
        .enumerate()
        .filter_map(|(index, text)| {
            let number = index as u64 + 1;
            (number >= start && number <= end).then(|| crate::model::CodeContextLine {
                number: Some(number),
                kind: crate::model::CodeLineKind::Context,
                text: text.to_owned(),
            })
        })
        .collect()
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_path_segment(segment: &str) -> String {
    segment
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn splits_repo_names() {
        assert_eq!(split_repo("owner/name").unwrap(), ("owner", "name"));
        assert!(split_repo("owner").is_err());
        assert!(split_repo("owner/").is_err());
    }

    #[test]
    fn source_context_lines_centers_around_highlighted_line() {
        let text = (1..=100)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");

        let lines = source_context_lines(&text, 75);

        assert_eq!(lines.first().and_then(|line| line.number), Some(65));
        assert_eq!(lines.last().and_then(|line| line.number), Some(85));
        assert!(lines.iter().any(|line| line.number == Some(75)));
    }

    #[test]
    fn source_context_lines_clamps_to_file_start() {
        let lines = source_context_lines("one\ntwo\nthree", 2);

        assert_eq!(lines.first().and_then(|line| line.number), Some(1));
        assert_eq!(lines.last().and_then(|line| line.number), Some(3));
    }

    #[test]
    fn encodes_content_api_paths() {
        assert_eq!(encode_path("src/main.rs"), "src/main.rs");
        assert_eq!(encode_path("docs/my file#.md"), "docs/my%20file%23.md");
    }
}
