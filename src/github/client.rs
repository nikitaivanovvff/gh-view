#[path = "gh/types.rs"]
mod types;

use self::types::{
    DashboardResponse, DashboardSearchResponse, DetailPullRequest, ReviewThreadsResponse,
    SearchPullRequest,
};
use super::{GhError, GhStatus, PullRequestSource};
use crate::github::command::{CommandRunner, GhCommand, command_error};
use crate::github::queries::{
    DETAIL_FIELDS, SEARCH_FIELDS, dashboard_query, dashboard_search_query, review_threads_query,
    split_repo,
};
use crate::model::{DiscussionItem, PullRequest, PullRequestDetail};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use std::{collections::HashSet, sync::Arc, time::Duration};

fn should_fallback_to_search(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| matches!(cause.downcast_ref::<GhError>(), Some(GhError::Command(_))))
}

#[derive(Clone)]
pub struct GhClient {
    command: Arc<dyn CommandRunner>,
}

impl GhClient {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            command: Arc::new(GhCommand::new(Duration::from_secs(timeout_seconds.max(1)))),
        }
    }

    #[cfg(test)]
    fn with_runner(command: Arc<dyn CommandRunner>) -> Self {
        Self { command }
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
        let mut cursor = None;
        let mut prs = Vec::new();
        loop {
            let response: DashboardSearchResponse =
                self.graphql(dashboard_search_query(query, cursor.as_deref()))?;
            let (page, next_cursor) = response.into_page();
            prs.extend(page);
            cursor = next_cursor;
            if cursor.is_none() {
                deduplicate_prs(&mut prs);
                return Ok(prs);
            }
        }
    }

    fn dashboard_graphql(&self, login: &str) -> Result<(Vec<PullRequest>, Vec<PullRequest>)> {
        let mut my_cursor = Some(None::<String>);
        let mut review_cursor = Some(None::<String>);
        let mut my_prs = Vec::new();
        let mut reviews = Vec::new();
        while my_cursor.is_some() || review_cursor.is_some() {
            let response: DashboardResponse = self.graphql(dashboard_query(
                login,
                my_cursor.as_ref().map(|cursor| cursor.as_deref()),
                review_cursor.as_ref().map(|cursor| cursor.as_deref()),
            ))?;
            let ((my_page, next_my_cursor), (review_page, next_review_cursor)) =
                response.into_page(login);
            my_prs.extend(my_page);
            reviews.extend(review_page);
            if my_cursor.is_some() {
                my_cursor = next_my_cursor.map(Some);
            }
            if review_cursor.is_some() {
                review_cursor = next_review_cursor.map(Some);
            }
        }
        deduplicate_prs(&mut my_prs);
        deduplicate_prs(&mut reviews);
        Ok((my_prs, reviews))
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
        let owner_field = format!("owner={owner}");
        let name_field = format!("name={name}");
        let number_field = format!("number={number}");

        let mut cursor = None;
        let mut items = Vec::new();
        loop {
            let query_field = format!("query={}", review_threads_query(cursor.as_deref()));
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
            let (page, next_cursor) = response.into_page();
            items.extend(page);
            cursor = next_cursor;
            if cursor.is_none() {
                return Ok(items);
            }
        }
    }
}

fn deduplicate_prs(prs: &mut Vec<PullRequest>) {
    let mut seen = HashSet::new();
    prs.retain(|pr| seen.insert((pr.repo.clone(), pr.number)));
}

impl PullRequestSource for GhClient {
    fn clone_box(&self) -> Box<dyn PullRequestSource> {
        Box::new(self.clone())
    }

    fn status(&self) -> GhStatus {
        let Ok(output) = self.run_gh(["--version"]) else {
            return GhStatus::Missing;
        };
        if !output.status.success() {
            return GhStatus::Missing;
        }
        let Some(version) = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::trim)
            .map(str::to_owned)
        else {
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
        self.command.run(
            args.into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{GhClient, deduplicate_prs, should_fallback_to_search};
    use crate::github::command::CommandRunner;
    use crate::github::{GhError, PullRequestSource};
    use crate::model::PullRequest;
    use anyhow::{Context, Result, anyhow};
    use std::collections::VecDeque;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::sync::{Arc, Mutex};

    enum ScriptedResult {
        Output(Output),
        Error(GhError),
    }

    struct ScriptedRunner {
        results: Mutex<VecDeque<ScriptedResult>>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl ScriptedRunner {
        fn new(results: Vec<ScriptedResult>) -> Arc<Self> {
            Arc::new(Self {
                results: Mutex::new(results.into()),
                calls: Mutex::new(Vec::new()),
            })
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CommandRunner for ScriptedRunner {
        fn run(&self, args: Vec<String>) -> Result<Output> {
            self.calls.lock().unwrap().push(args);
            match self.results.lock().unwrap().pop_front().unwrap() {
                ScriptedResult::Output(output) => Ok(output),
                ScriptedResult::Error(error) => Err(anyhow!(error)),
            }
        }
    }

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

    #[test]
    fn dashboard_auth_failure_does_not_fall_back() {
        let runner = ScriptedRunner::new(vec![ScriptedResult::Output(failure(
            "authentication required; run gh auth login",
        ))]);
        let client = GhClient::with_runner(runner.clone());

        let error = client.fetch_dashboard("octocat").unwrap_err();

        assert!(matches!(
            error.downcast_ref(),
            Some(GhError::Unauthenticated(_))
        ));
        assert_graphql_call(&runner.calls()[0]);
        assert_eq!(runner.calls().len(), 1);
    }

    #[test]
    fn dashboard_timeout_does_not_fall_back() {
        let runner = ScriptedRunner::new(vec![ScriptedResult::Error(GhError::Timeout(
            "timed out".into(),
        ))]);
        let client = GhClient::with_runner(runner.clone());

        let error = client.fetch_dashboard("octocat").unwrap_err();

        assert!(matches!(error.downcast_ref(), Some(GhError::Timeout(_))));
        assert_graphql_call(&runner.calls()[0]);
        assert_eq!(runner.calls().len(), 1);
    }

    #[test]
    fn dashboard_command_failure_invokes_search_fallbacks() {
        let runner = ScriptedRunner::new(vec![
            ScriptedResult::Output(failure("GraphQL query failed")),
            ScriptedResult::Output(success("[]")),
            ScriptedResult::Output(success("[]")),
        ]);
        let client = GhClient::with_runner(runner.clone());

        let dashboard = client.fetch_dashboard("octocat").unwrap();

        assert_eq!(dashboard, (Vec::new(), Vec::new()));
        let calls = runner.calls();
        assert_graphql_call(&calls[0]);
        assert_eq!(calls[1], search_args("--author"));
        assert_eq!(calls[2], search_args("--review-requested"));
    }

    #[test]
    fn dashboard_fallback_failure_retains_graphql_context() {
        let runner = ScriptedRunner::new(vec![
            ScriptedResult::Output(failure("original GraphQL failure")),
            ScriptedResult::Output(failure("fallback search failure")),
        ]);
        let client = GhClient::with_runner(runner.clone());

        let error = client.fetch_dashboard("octocat").unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("original GraphQL failure"));
        assert!(message.contains("fallback search failure"));
        let calls = runner.calls();
        assert_graphql_call(&calls[0]);
        assert_eq!(calls[1], search_args("--author"));
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn combines_pages_without_duplicate_pull_requests() {
        let mut pages = vec![pr("owner/repo", 1), pr("owner/repo", 2)];
        pages.extend([pr("owner/repo", 2), pr("other/repo", 1)]);

        deduplicate_prs(&mut pages);

        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].number, 1);
        assert_eq!(pages[1].number, 2);
        assert_eq!(pages[2].repo, "other/repo");
    }

    fn pr(repo: &str, number: u64) -> PullRequest {
        PullRequest {
            repo: repo.to_owned(),
            number,
            title: String::new(),
            author: String::new(),
            head_ref: String::new(),
            url: String::new(),
            updated_at: String::new(),
            state: String::new(),
            is_draft: false,
            review_decision: None,
            check_status: None,
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }

    fn success(stdout: &str) -> Output {
        output(0, stdout, "")
    }

    fn failure(stderr: &str) -> Output {
        output(1, "", stderr)
    }

    fn output(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn search_args(filter: &str) -> Vec<String> {
        [
            "search",
            "prs",
            "--state",
            "open",
            "--json",
            crate::github::queries::SEARCH_FIELDS,
            filter,
            "octocat",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    fn assert_graphql_call(args: &[String]) {
        assert_eq!(&args[..3], ["api", "graphql", "-f"]);
        assert!(args[3].starts_with("query={\n"));
        assert!(args[3].contains("author:octocat"));
        assert!(args[3].contains("review-requested:octocat"));
    }
}
