#[path = "gh/types.rs"]
mod types;

use self::types::{
    DashboardResponse, DashboardSearchResponse, DetailPullRequest, ReviewThreadsResponse,
    SearchPullRequest,
};
use super::{GhError, GhStatus, PullRequestSource};
use crate::github::command::{GhCommand, command_error};
use crate::github::queries::{
    DETAIL_FIELDS, REVIEW_THREADS_QUERY, SEARCH_FIELDS, dashboard_query, dashboard_search_query,
    split_repo,
};
use crate::model::{DiscussionItem, PullRequest, PullRequestDetail};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use std::time::Duration;

fn should_fallback_to_search(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| matches!(cause.downcast_ref::<GhError>(), Some(GhError::Command(_))))
}

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
        let items = response.into_discussion_items();
        Ok(items)
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
        match self.search_dashboard_graphql(&format!("is:pr is:open author:{login} archived:false"))
        {
            Ok(prs) => Ok(prs),
            Err(graphql_error) if should_fallback_to_search(&graphql_error) => {
                self.search_prs(&["--author", login]).with_context(|| {
                    format!("search fallback failed after GraphQL failure: {graphql_error}")
                })
            }
            Err(error) => Err(error),
        }
    }

    fn fetch_dashboard(&self, login: &str) -> Result<(Vec<PullRequest>, Vec<PullRequest>)> {
        match self.dashboard_graphql(login) {
            Ok(dashboard) => Ok(dashboard),
            Err(graphql_error) if should_fallback_to_search(&graphql_error) => (|| -> Result<_> {
                Ok((
                    self.search_prs(&["--author", login])?,
                    self.search_prs(&["--review-requested", login])?,
                ))
            })()
            .with_context(|| {
                format!("search fallback failed after GraphQL failure: {graphql_error}")
            }),
            Err(error) => Err(error),
        }
    }

    fn fetch_review_requests(&self, login: &str) -> Result<Vec<PullRequest>> {
        let graphql_result = self.search_dashboard_graphql(&format!(
            "is:pr is:open review-requested:{login} archived:false"
        ));

        match graphql_result {
            Ok(prs) => Ok(prs
                .into_iter()
                .filter(|pr| pr.review_requested.iter().any(|reviewer| reviewer == login))
                .collect()),
            Err(graphql_error) if should_fallback_to_search(&graphql_error) => self
                .search_prs(&["--review-requested", login])
                .with_context(|| {
                    format!("search fallback failed after GraphQL failure: {graphql_error}")
                }),
            Err(error) => Err(error),
        }
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

impl GhClient {
    fn run_gh<I, S>(&self, args: I) -> Result<std::process::Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.command.run(args)
    }
}

#[cfg(test)]
mod tests {
    use super::should_fallback_to_search;
    use crate::github::GhError;
    use anyhow::{Context, anyhow};

    #[test]
    fn search_fallback_accepts_command_error_in_chain() {
        let error = Err::<(), _>(GhError::Command("unsupported query".into()))
            .context("GraphQL request failed")
            .unwrap_err();

        assert!(should_fallback_to_search(&error));
    }

    #[test]
    fn search_fallback_rejects_preserved_error_categories() {
        let errors = [
            GhError::Missing("missing".into()),
            GhError::Unauthenticated("unauthenticated".into()),
            GhError::GitHubOutage("outage".into()),
            GhError::Timeout("timeout".into()),
        ];

        for error in errors {
            assert!(!should_fallback_to_search(&anyhow!(error)));
        }
    }

    #[test]
    fn search_fallback_rejects_non_gh_errors() {
        let error = anyhow!("failed to parse gh GraphQL output").context("malformed JSON");

        assert!(!should_fallback_to_search(&error));
    }
}
