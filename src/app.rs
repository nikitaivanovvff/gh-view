mod rows;

use crate::github::{
    GhError, MockErrorMode, PullRequestSource, is_auth_error, is_github_outage_error,
};
use crate::model::{Dashboard, PullRequest, PullRequestDetail};
pub use rows::{DashboardSection, Row};
use rows::{group_names, push_groups};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver};
use std::thread;

type DashboardLoad = Result<(String, Vec<PullRequest>, Vec<PullRequest>), AppStatus>;
type DashboardReceiver = Receiver<DashboardLoad>;

pub struct App {
    client: Box<dyn PullRequestSource>,
    dashboard: Dashboard,
    collapsed_groups: BTreeSet<String>,
    pub current_user: Option<String>,
    pub selected: usize,
    pub status: AppStatus,
    pub view: AppView,
    pub detail: Option<PullRequestDetail>,
    pub detail_scroll: u16,
    pub discussion_selected: usize,
    pub discussion_scroll: u16,
    pub active_detail_pane: DetailPane,
    pub detail_status: DetailStatus,
    pub discussion_status: DiscussionStatus,
    pub dashboard_loading: bool,
    pub loading_frame: usize,
    dashboard_rx: Option<DashboardReceiver>,
    detail_rx: Option<Receiver<Result<PullRequestDetail, String>>>,
    discussion_rx: Option<Receiver<Result<Vec<crate::model::DiscussionItem>, String>>>,
}

impl App {
    pub fn new(client: Box<dyn PullRequestSource>) -> Self {
        Self {
            client,
            dashboard: Dashboard::default(),
            collapsed_groups: BTreeSet::new(),
            current_user: None,
            selected: 0,
            status: AppStatus::Ready,
            view: AppView::Dashboard,
            detail: None,
            detail_scroll: 0,
            discussion_selected: 0,
            discussion_scroll: 0,
            active_detail_pane: DetailPane::Description,
            detail_status: DetailStatus::Idle,
            discussion_status: DiscussionStatus::Idle,
            dashboard_loading: false,
            loading_frame: 0,
            dashboard_rx: None,
            detail_rx: None,
            discussion_rx: None,
        }
    }

    #[cfg(test)]
    pub fn refresh(&mut self) {
        let result = refresh_dashboard(self.client.clone(), self.current_user.clone());
        self.apply_refresh_result(result.map_err(classify_refresh_error));
    }

    pub fn refresh_async(&mut self) {
        if self.dashboard_loading {
            return;
        }

        let client = self.client.clone();
        let current_user = self.current_user.clone();
        let (tx, rx) = mpsc::channel();
        self.dashboard_rx = Some(rx);
        self.dashboard_loading = true;
        self.loading_frame = 0;

        thread::spawn(move || {
            let result = refresh_dashboard(client, current_user).map_err(classify_refresh_error);
            let _ = tx.send(result);
        });
    }

    fn apply_refresh_result(&mut self, result: DashboardLoad) {
        self.dashboard_loading = false;
        self.dashboard_rx = None;
        match result {
            Ok((user, my_prs, reviews)) => {
                self.current_user = Some(user);
                self.dashboard = Dashboard::from_prs(my_prs, reviews);
                let valid_groups = group_names(&self.dashboard);
                self.collapsed_groups = self
                    .collapsed_groups
                    .intersection(&valid_groups)
                    .cloned()
                    .collect();
                self.status = AppStatus::Ready;
                self.clamp_selection();
            }
            Err(status) => {
                self.status = status;
                self.dashboard = Dashboard::default();
            }
        }
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        match &self.status {
            AppStatus::MissingGh => return vec![Row::Message("GitHub CLI `gh` was not found on PATH. Install it, authenticate it, then press r to retry.".to_owned())],
            AppStatus::Unauthenticated(_) => return vec![Row::Message("GitHub CLI is not authenticated. Run `gh auth login`, then press r to retry.".to_owned())],
            AppStatus::GitHubOutage(_) if self.dashboard.is_empty() => return github_outage_rows(),
            AppStatus::Timeout(_) if self.dashboard.is_empty() => return vec![
                Row::Message("GitHub is taking too long to answer.".to_owned()),
                Row::Message("The last gh command was stopped after 30s. Press r to retry.".to_owned()),
            ],
            AppStatus::Error(_) if self.dashboard.is_empty() => return vec![Row::Message("Could not load pull requests. Press r to retry.".to_owned())],
            _ => {}
        }

        let mut rows = Vec::new();
        rows.push(Row::Section("My PRs"));
        push_groups(
            &mut rows,
            DashboardSection::MyPrs,
            &self.dashboard.my_prs,
            &self.collapsed_groups,
        );
        rows.push(Row::Section("Awaiting Review"));
        push_groups(
            &mut rows,
            DashboardSection::AwaitingReview,
            &self.dashboard.awaiting_review,
            &self.collapsed_groups,
        );

        if self.dashboard.is_empty() {
            rows.push(Row::Message(
                "No open PRs found. Press r to refresh.".to_owned(),
            ));
        }

        rows
    }

    pub fn clamp_selection(&mut self) {
        self.selected = self.selected.min(self.rows().len().saturating_sub(1));
    }

    pub fn next(&mut self) {
        let len = self.rows().len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_selected_group(&mut self) {
        let rows = self.rows();
        let Some(repo) = rows.get(self.selected).and_then(Row::group_key) else {
            return;
        };

        if !self.collapsed_groups.insert(repo.clone()) {
            self.collapsed_groups.remove(&repo);
        }
    }

    pub fn open_selected_detail(&mut self) {
        let rows = self.rows();
        let Some(pr) = rows.get(self.selected).and_then(Row::pr).cloned() else {
            return;
        };

        self.open_detail_for_pr(pr);
    }

    fn open_detail_for_pr(&mut self, pr: PullRequest) {
        self.detail = Some(placeholder_detail(pr.clone()));
        self.view = AppView::Detail;
        self.detail_scroll = 0;
        self.discussion_selected = 0;
        self.discussion_scroll = 0;
        self.active_detail_pane = DetailPane::Description;
        self.status = AppStatus::Ready;
        self.start_detail_load(pr.clone());
        self.start_discussion_load(pr);
    }

    pub fn poll_background(&mut self) -> bool {
        let mut changed = false;
        changed |= self.poll_dashboard_load();
        changed |= self.poll_detail_load();
        changed |= self.poll_discussion_load();
        changed
    }

    fn poll_dashboard_load(&mut self) -> bool {
        let Some(rx) = &self.dashboard_rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.apply_refresh_result(result);
        true
    }

    fn poll_detail_load(&mut self) -> bool {
        let Some(rx) = &self.detail_rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.detail_rx = None;
        match result {
            Ok(mut loaded_detail) => {
                if let Some(current_detail) = &mut self.detail {
                    let mut existing_review_threads: Vec<_> = current_detail
                        .discussion
                        .iter()
                        .filter(|item| {
                            matches!(item.kind, crate::model::DiscussionKind::ReviewThread { .. })
                        })
                        .cloned()
                        .collect();
                    if loaded_detail
                        .pr
                        .review_decision
                        .as_deref()
                        .is_none_or(str::is_empty)
                    {
                        loaded_detail.pr.review_decision =
                            current_detail.pr.review_decision.clone();
                    }
                    if loaded_detail
                        .pr
                        .check_status
                        .as_deref()
                        .is_none_or(str::is_empty)
                    {
                        loaded_detail.pr.check_status = current_detail.pr.check_status.clone();
                    }
                    loaded_detail
                        .discussion
                        .append(&mut existing_review_threads);
                    loaded_detail.discussion.sort_by(|left, right| {
                        left.created_at
                            .cmp(&right.created_at)
                            .then_with(|| left.url.cmp(&right.url))
                    });
                    *current_detail = loaded_detail;
                }
                self.detail_status = DetailStatus::Ready;
            }
            Err(error) => {
                self.detail_status = DetailStatus::Error(error);
            }
        }
        true
    }

    fn poll_discussion_load(&mut self) -> bool {
        let Some(rx) = &self.discussion_rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.discussion_rx = None;
        match result {
            Ok(mut discussion) => {
                if let Some(detail) = &mut self.detail {
                    detail.discussion.append(&mut discussion);
                    detail.discussion.sort_by(|left, right| {
                        left.created_at
                            .cmp(&right.created_at)
                            .then_with(|| left.url.cmp(&right.url))
                    });
                }
                self.discussion_status = DiscussionStatus::Ready;
            }
            Err(error) => {
                self.discussion_status = DiscussionStatus::Error(error);
            }
        }
        true
    }

    fn start_detail_load(&mut self, pr: PullRequest) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel();
        self.detail_rx = Some(rx);
        self.detail_status = DetailStatus::Loading;
        self.loading_frame = 0;

        thread::spawn(move || {
            let result = client
                .fetch_pr_detail(&pr)
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    fn start_discussion_load(&mut self, pr: PullRequest) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel();
        self.discussion_rx = Some(rx);
        self.discussion_status = DiscussionStatus::Loading;
        self.loading_frame = 0;

        thread::spawn(move || {
            let result = client
                .fetch_pr_discussion(&pr)
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    pub fn back_to_dashboard(&mut self) {
        self.view = AppView::Dashboard;
        self.detail = None;
        self.detail_scroll = 0;
        self.discussion_selected = 0;
        self.discussion_scroll = 0;
        self.active_detail_pane = DetailPane::Description;
        self.detail_status = DetailStatus::Idle;
        self.discussion_status = DiscussionStatus::Idle;
        self.detail_rx = None;
        self.discussion_rx = None;
    }

    pub fn scroll_active_down(&mut self) {
        match self.active_detail_pane {
            DetailPane::Description => self.detail_scroll = self.detail_scroll.saturating_add(1),
            DetailPane::Discussion => {
                self.discussion_scroll = self.discussion_scroll.saturating_add(1);
            }
        }
    }

    pub fn scroll_active_up(&mut self) {
        match self.active_detail_pane {
            DetailPane::Description => self.detail_scroll = self.detail_scroll.saturating_sub(1),
            DetailPane::Discussion => {
                self.discussion_scroll = self.discussion_scroll.saturating_sub(1);
            }
        }
    }

    pub fn toggle_detail_pane(&mut self) {
        self.active_detail_pane = match self.active_detail_pane {
            DetailPane::Description => DetailPane::Discussion,
            DetailPane::Discussion => DetailPane::Description,
        };
    }

    pub fn next_discussion(&mut self) {
        let Some(detail) = &self.detail else {
            return;
        };
        if !detail.discussion.is_empty() {
            self.discussion_selected = (self.discussion_selected + 1) % detail.discussion.len();
            self.discussion_scroll = 0;
        }
    }

    pub fn previous_discussion(&mut self) {
        let Some(detail) = &self.detail else {
            return;
        };
        if !detail.discussion.is_empty() {
            self.discussion_selected = if self.discussion_selected == 0 {
                detail.discussion.len() - 1
            } else {
                self.discussion_selected - 1
            };
            self.discussion_scroll = 0;
        }
    }

    pub fn selected_discussion_index(&self) -> usize {
        self.detail
            .as_ref()
            .filter(|detail| !detail.discussion.is_empty())
            .map(|detail| self.discussion_selected.min(detail.discussion.len() - 1))
            .unwrap_or(0)
    }

    pub fn open_selected_in_browser(&mut self) {
        let url = match self.view {
            AppView::Dashboard => self
                .rows()
                .get(self.selected)
                .and_then(Row::pr_url)
                .map(str::to_owned),
            AppView::Detail => self.detail.as_ref().map(|detail| detail.pr.url.clone()),
        };

        let Some(url) = url else {
            return;
        };

        if let Err(error) = webbrowser::open(&url) {
            self.status = AppStatus::Error(format!("failed to open browser: {error}"));
        }
    }

    pub fn show_dashboard_loading_screen(&self) -> bool {
        self.dashboard_loading && self.dashboard.is_empty()
    }

    pub fn detail_is_loading(&self) -> bool {
        self.detail_status == DetailStatus::Loading
            || self.discussion_status == DiscussionStatus::Loading
    }

    pub fn is_loading(&self) -> bool {
        self.dashboard_loading || self.detail_is_loading()
    }

    pub fn advance_loading_frame(&mut self) {
        if self.is_loading() {
            self.loading_frame = self.loading_frame.wrapping_add(1);
        }
    }

    pub fn dashboard_error_page(&self) -> Option<DashboardErrorPage> {
        if !self.dashboard.is_empty() {
            return None;
        }

        match self.status {
            AppStatus::MissingGh => Some(DashboardErrorPage {
                art: Vec::new(),
                lines: vec![
                    DashboardErrorLine::Text("GitHub CLI `gh` was not found on PATH.".to_owned()),
                    DashboardErrorLine::Text(
                        "Install it, authenticate it, then press r to retry.".to_owned(),
                    ),
                ],
            }),
            AppStatus::Unauthenticated(_) => Some(DashboardErrorPage {
                art: Vec::new(),
                lines: vec![
                    DashboardErrorLine::Text("GitHub CLI is not authenticated.".to_owned()),
                    DashboardErrorLine::Text(
                        "Run `gh auth login`, then press r to retry.".to_owned(),
                    ),
                ],
            }),
            AppStatus::GitHubOutage(_) => Some(DashboardErrorPage {
                art: vec![
                    "        Zzz".to_owned(),
                    "     /\\_/\\".to_owned(),
                    "    ( -.- )".to_owned(),
                    "    / >  < \\".to_owned(),
                    "GitHub cat is asleep on the deploy button".to_owned(),
                ],
                lines: vec![
                    DashboardErrorLine::Text("Looks like GitHub is having a problem.".to_owned()),
                    DashboardErrorLine::StatusPage,
                ],
            }),
            AppStatus::Timeout(_) => Some(DashboardErrorPage {
                art: Vec::new(),
                lines: vec![
                    DashboardErrorLine::Text("GitHub is taking too long to answer.".to_owned()),
                    DashboardErrorLine::Text(
                        "The last gh command was stopped. Press r to retry.".to_owned(),
                    ),
                ],
            }),
            AppStatus::Error(_) => Some(DashboardErrorPage {
                art: Vec::new(),
                lines: vec![DashboardErrorLine::Text(
                    "Could not load pull requests. Press r to retry.".to_owned(),
                )],
            }),
            AppStatus::Ready => None,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.client.is_mock()
    }

    pub fn mock_error_mode(&self) -> Option<MockErrorMode> {
        self.client.mock_error_mode()
    }

    pub fn set_mock_error_mode(&mut self, mode: Option<MockErrorMode>) {
        if !self.client.is_mock() {
            return;
        }
        self.client.set_mock_error_mode(mode);
        self.dashboard_loading = false;
        self.dashboard_rx = None;
        self.refresh_async();
    }
}

fn refresh_dashboard(
    client: Box<dyn PullRequestSource>,
    current_user: Option<String>,
) -> anyhow::Result<(String, Vec<PullRequest>, Vec<PullRequest>)> {
    let user = match current_user {
        Some(user) => user,
        None => client.current_user()?,
    };

    let (my_prs, reviews) = client.fetch_dashboard(&user)?;

    Ok((user, my_prs, reviews))
}

fn classify_refresh_error(error: anyhow::Error) -> AppStatus {
    if let Some(error) = error.downcast_ref::<GhError>() {
        return match error {
            GhError::Missing(_) => AppStatus::MissingGh,
            GhError::Unauthenticated(message) => AppStatus::Unauthenticated(message.clone()),
            GhError::GitHubOutage(message) => AppStatus::GitHubOutage(message.clone()),
            GhError::Timeout(message) => AppStatus::Timeout(message.clone()),
            GhError::Command(message) => classify_refresh_message(message.clone()),
        };
    }

    classify_refresh_message(error.to_string())
}

fn classify_refresh_message(message: String) -> AppStatus {
    if message.contains("executable file not found")
        || message.contains("No such file or directory")
        || message.contains("failed to run gh")
    {
        AppStatus::MissingGh
    } else if is_auth_error(&message) {
        AppStatus::Unauthenticated(message)
    } else if is_github_outage_error(&message) {
        AppStatus::GitHubOutage(message)
    } else if message.contains("timed out after ") {
        AppStatus::Timeout(message)
    } else {
        AppStatus::Error(message)
    }
}

fn github_outage_rows() -> Vec<Row<'static>> {
    vec![
        Row::Message("        Zzz".to_owned()),
        Row::Message("     /\\_/\\".to_owned()),
        Row::Message("    ( -.- )".to_owned()),
        Row::Message("    / >  < \\".to_owned()),
        Row::Message("GitHub cat is asleep on the deploy button".to_owned()),
        Row::Message(String::new()),
        Row::Message("Looks like GitHub is having a problem.".to_owned()),
        Row::Message("Check https://www.githubstatus.com/ and press r to retry.".to_owned()),
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardErrorPage {
    pub art: Vec<String>,
    pub lines: Vec<DashboardErrorLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DashboardErrorLine {
    Text(String),
    StatusPage,
}

fn placeholder_detail(pr: PullRequest) -> PullRequestDetail {
    let state = if pr.state.is_empty() {
        "unknown".to_owned()
    } else {
        pr.state.clone()
    };

    PullRequestDetail {
        pr,
        body: String::new(),
        state,
        mergeable: None,
        head_ref: String::new(),
        base_ref: String::new(),
        reviews: Vec::new(),
        discussion: Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppView {
    Dashboard,
    Detail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStatus {
    Ready,
    MissingGh,
    Unauthenticated(String),
    GitHubOutage(String),
    Timeout(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DetailPane {
    Description,
    Discussion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DetailStatus {
    Idle,
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscussionStatus {
    Idle,
    Loading,
    Ready,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::GhStatus;
    use crate::model::{DiscussionItem, DiscussionKind, Reviewer, ReviewerState};
    use anyhow::{Result, anyhow};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    struct TestSource {
        current_user: Result<String, String>,
        my_prs: Result<Vec<PullRequest>, String>,
        review_prs: Result<Vec<PullRequest>, String>,
        detail: Result<PullRequestDetail, String>,
        discussion: Result<Vec<DiscussionItem>, String>,
        current_user_calls: Arc<AtomicUsize>,
    }

    impl TestSource {
        fn ok() -> Self {
            let main_pr = pr("owner/repo", 1);
            Self {
                current_user: Ok("octocat".to_owned()),
                my_prs: Ok(vec![main_pr.clone()]),
                review_prs: Ok(vec![pr("owner/other", 2)]),
                detail: Ok(detail(main_pr)),
                discussion: Ok(vec![discussion("alice", "2026-07-01T10:00:00Z")]),
                current_user_calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl PullRequestSource for TestSource {
        fn clone_box(&self) -> Box<dyn PullRequestSource> {
            Box::new(self.clone())
        }

        fn status(&self) -> GhStatus {
            GhStatus::Ready {
                version: "test-gh".to_owned(),
            }
        }

        fn current_user(&self) -> Result<String> {
            self.current_user_calls.fetch_add(1, Ordering::SeqCst);
            self.current_user
                .clone()
                .map_err(|message| anyhow!(message))
        }

        fn fetch_my_prs(&self, _login: &str) -> Result<Vec<PullRequest>> {
            self.my_prs.clone().map_err(|message| anyhow!(message))
        }

        fn fetch_review_requests(&self, _login: &str) -> Result<Vec<PullRequest>> {
            self.review_prs.clone().map_err(|message| anyhow!(message))
        }

        fn fetch_pr_detail(&self, _pr: &PullRequest) -> Result<PullRequestDetail> {
            self.detail.clone().map_err(|message| anyhow!(message))
        }

        fn fetch_pr_discussion(&self, _pr: &PullRequest) -> Result<Vec<DiscussionItem>> {
            self.discussion.clone().map_err(|message| anyhow!(message))
        }
    }

    #[test]
    fn refresh_loads_dashboard_and_caches_current_user() {
        let source = TestSource::ok();
        let calls = source.current_user_calls.clone();
        let mut app = App::new(Box::new(source));

        app.refresh();
        app.refresh();

        assert_eq!(app.current_user.as_deref(), Some("octocat"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(matches!(app.status, AppStatus::Ready));
        assert!(
            app.rows()
                .iter()
                .any(|row| matches!(row, Row::Pr(pr) if pr.number == 1))
        );
        assert!(
            app.rows()
                .iter()
                .any(|row| matches!(row, Row::Pr(pr) if pr.number == 2))
        );
    }

    #[test]
    fn refresh_classifies_missing_gh_errors() {
        let mut source = TestSource::ok();
        source.current_user = Err("failed to run gh api user".to_owned());
        let mut app = App::new(Box::new(source));

        app.refresh();

        assert_eq!(app.status, AppStatus::MissingGh);
        assert!(
            matches!(app.rows().first(), Some(Row::Message(message)) if message.contains("not found"))
        );
    }

    #[test]
    fn refresh_classifies_auth_errors() {
        let mut source = TestSource::ok();
        source.current_user = Err("authentication required; run gh auth login".to_owned());
        let mut app = App::new(Box::new(source));

        app.refresh();

        assert!(matches!(app.status, AppStatus::Unauthenticated(_)));
    }

    #[test]
    fn refresh_classifies_github_outage_errors() {
        let mut source = TestSource::ok();
        source.current_user = Err("HTTP 503 Service Unavailable".to_owned());
        let mut app = App::new(Box::new(source));

        app.refresh();

        assert!(matches!(app.status, AppStatus::GitHubOutage(_)));
        let text = app
            .rows()
            .into_iter()
            .map(|row| match row {
                Row::Message(message) => message,
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("GitHub cat is asleep"));
        assert!(text.contains("githubstatus.com"));
    }

    #[test]
    fn refresh_classifies_timeout_errors() {
        let mut source = TestSource::ok();
        source.current_user = Err("gh command timed out after 30s: gh api graphql".to_owned());
        let mut app = App::new(Box::new(source));

        app.refresh();

        assert!(matches!(app.status, AppStatus::Timeout(_)));
        assert!(matches!(
            app.rows().first(),
            Some(Row::Message(message)) if message.contains("taking too long")
        ));
    }

    #[test]
    fn dashboard_loading_screen_is_for_empty_dashboard_loads() {
        let mut app = App::new(Box::new(TestSource::ok()));

        app.dashboard_loading = true;
        assert!(app.show_dashboard_loading_screen());
        assert!(matches!(
            app.rows().last(),
            Some(Row::Message(message)) if !message.contains("Loading")
        ));

        app.current_user = Some("octocat".to_owned());
        assert!(app.show_dashboard_loading_screen());

        app.dashboard_loading = false;
        app.refresh();
        app.dashboard_loading = true;
        assert!(!app.show_dashboard_loading_screen());
    }

    #[test]
    fn loading_frame_advances_only_while_loading() {
        let mut app = App::new(Box::new(TestSource::ok()));

        app.advance_loading_frame();
        assert_eq!(app.loading_frame, 0);

        app.dashboard_loading = true;
        app.advance_loading_frame();
        assert_eq!(app.loading_frame, 1);
    }

    #[test]
    fn same_repo_groups_collapse_independently_across_sections() {
        let mut source = TestSource::ok();
        source.my_prs = Ok(vec![pr("owner/shared", 1)]);
        source.review_prs = Ok(vec![pr("owner/shared", 2)]);
        let mut app = App::new(Box::new(source));
        app.refresh();

        app.next();
        app.toggle_selected_group();

        let rows = app.rows();
        assert!(matches!(
            rows.get(1),
            Some(Row::Group {
                repo: "owner/shared",
                open: false,
                ..
            })
        ));
        assert!(matches!(
            rows.get(3),
            Some(Row::Group {
                repo: "owner/shared",
                open: true,
                ..
            })
        ));
    }

    #[test]
    fn navigation_and_group_toggle_update_visible_rows() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();

        let expanded_count = app.rows().len();
        app.next();
        assert_eq!(app.selected, 1);
        app.toggle_selected_group();

        assert!(app.rows().len() < expanded_count);
        assert!(matches!(
            app.rows().get(1),
            Some(Row::Group { open: false, .. })
        ));
        app.previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn opening_detail_uses_placeholder_then_background_results() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();
        app.next();
        app.next();
        app.open_selected_detail();

        assert_eq!(app.view, AppView::Detail);
        assert_eq!(app.detail_status, DetailStatus::Loading);
        assert_eq!(app.discussion_status, DiscussionStatus::Loading);
        assert_eq!(app.detail.as_ref().unwrap().pr.number, 1);

        poll_until_ready(&mut app);

        assert_eq!(app.detail_status, DetailStatus::Ready);
        assert_eq!(app.discussion_status, DiscussionStatus::Ready);
        let detail = app.detail.as_ref().unwrap();
        assert_eq!(detail.body, "Loaded body");
        assert_eq!(detail.discussion.len(), 2);
    }

    #[test]
    fn detail_load_preserves_dashboard_review_and_check_metadata_when_empty() {
        let mut source = TestSource::ok();
        let mut loaded = detail(pr("owner/repo", 1));
        loaded.pr.review_decision = Some(String::new());
        loaded.pr.check_status = Some(String::new());
        source.detail = Ok(loaded);
        let mut app = App::new(Box::new(source));

        app.refresh();
        app.next();
        app.next();
        app.open_selected_detail();
        poll_until_ready(&mut app);

        let pr = &app.detail.as_ref().unwrap().pr;
        assert_eq!(pr.review_decision.as_deref(), Some("APPROVED"));
        assert_eq!(pr.check_status.as_deref(), Some("passing"));
    }

    #[test]
    fn detail_pane_focus_controls_active_scroll_target() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.scroll_active_down();
        assert_eq!(app.detail_scroll, 1);
        assert_eq!(app.discussion_scroll, 0);

        app.toggle_detail_pane();
        app.scroll_active_down();
        assert_eq!(app.detail_scroll, 1);
        assert_eq!(app.discussion_scroll, 1);

        app.toggle_detail_pane();
        assert_eq!(app.active_detail_pane, DetailPane::Description);
        app.scroll_active_up();
        assert_eq!(app.detail_scroll, 0);
    }

    #[test]
    fn discussion_selection_wraps_and_resets_scroll() {
        let mut app = App::new(Box::new(TestSource::ok()));
        let mut detail = detail(pr("owner/repo", 1));
        detail.discussion = vec![
            discussion("alice", "2026-07-01T10:00:00Z"),
            discussion("bob", "2026-07-01T10:01:00Z"),
        ];
        app.detail = Some(detail);
        app.discussion_scroll = 4;

        app.previous_discussion();
        assert_eq!(app.selected_discussion_index(), 1);
        assert_eq!(app.discussion_scroll, 0);
        app.next_discussion();
        assert_eq!(app.selected_discussion_index(), 0);
    }

    fn poll_until_ready(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            app.poll_background();
            if app.detail_status != DetailStatus::Loading
                && app.discussion_status != DiscussionStatus::Loading
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("background loads did not finish");
    }

    fn pr(repo: &str, number: u64) -> PullRequest {
        PullRequest {
            repo: repo.to_owned(),
            number,
            title: format!("PR {number}"),
            author: "author".to_owned(),
            url: format!("https://github.com/{repo}/pull/{number}"),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
            state: "OPEN".to_owned(),
            is_draft: false,
            review_decision: Some("APPROVED".to_owned()),
            check_status: Some("passing".to_owned()),
            reviewers: vec![Reviewer {
                login: "reviewer".to_owned(),
                state: ReviewerState::Approved,
            }],
            review_requested: vec!["reviewer".to_owned()],
        }
    }

    fn detail(pr: PullRequest) -> PullRequestDetail {
        PullRequestDetail {
            pr,
            body: "Loaded body".to_owned(),
            state: "OPEN".to_owned(),
            mergeable: Some("MERGEABLE".to_owned()),
            head_ref: "feature".to_owned(),
            base_ref: "main".to_owned(),
            reviews: Vec::new(),
            discussion: vec![discussion("carol", "2026-07-01T09:00:00Z")],
        }
    }

    fn discussion(author: &str, created_at: &str) -> DiscussionItem {
        DiscussionItem {
            kind: DiscussionKind::IssueComment,
            author: author.to_owned(),
            body: "comment".to_owned(),
            created_at: created_at.to_owned(),
            url: format!("https://example.test/{author}"),
            replies: Vec::new(),
            code_context: None,
        }
    }
}
