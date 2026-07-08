mod dashboard;
mod detail;
mod rows;
mod search;
mod status;

use crate::github::{MockErrorMode, PullRequestSource};
use crate::model::PullRequest;
use dashboard::DashboardLoad;
pub use dashboard::DashboardState;
pub use detail::{DetailPane, DetailState, DetailStatus, DiscussionStatus};
pub use rows::DashboardSection;
pub use rows::Row;
pub use search::DashboardSearchMatch;
use status::classify_refresh_error;
pub use status::{AppStatus, DashboardErrorLine, DashboardErrorPage, pull_request_status};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const COPY_NOTICE_DURATION: Duration = Duration::from_secs(5);

pub struct App {
    client: Box<dyn PullRequestSource>,
    pub dashboard: DashboardState,
    pub status: AppStatus,
    pub view: AppView,
    pub detail: DetailState,
    pub loading_frame: usize,
    nerd_fonts: bool,
    copy_notice: Option<CopyNotice>,
}

struct CopyNotice {
    message: String,
    expires_at: Instant,
}

impl App {
    #[cfg(test)]
    pub fn new(client: Box<dyn PullRequestSource>) -> Self {
        Self::with_options(client, false, crate::app::rows::DEFAULT_PRS_PER_REPO_PAGE)
    }

    pub fn with_options(
        client: Box<dyn PullRequestSource>,
        nerd_fonts: bool,
        dashboard_prs_per_repo_page: usize,
    ) -> Self {
        Self {
            client,
            dashboard: DashboardState::with_page_size(dashboard_prs_per_repo_page),
            status: AppStatus::Ready,
            view: AppView::Dashboard,
            detail: DetailState::new(),
            loading_frame: 0,
            nerd_fonts,
            copy_notice: None,
        }
    }

    pub fn nerd_fonts(&self) -> bool {
        self.nerd_fonts
    }

    #[cfg(test)]
    pub fn refresh(&mut self) {
        let result = refresh_dashboard(self.client.clone(), self.dashboard.current_user.clone());
        self.apply_refresh_result(result.map_err(classify_refresh_error));
    }

    pub fn refresh_async(&mut self) {
        if self.dashboard.loading {
            return;
        }

        let client = self.client.clone();
        let current_user = self.dashboard.current_user.clone();
        let (tx, rx) = mpsc::channel();
        self.dashboard.rx = Some(rx);
        self.dashboard.loading = true;
        self.loading_frame = 0;

        thread::spawn(move || {
            let result = refresh_dashboard(client, current_user).map_err(classify_refresh_error);
            let _ = tx.send(result);
        });
    }

    fn apply_refresh_result(&mut self, result: DashboardLoad) {
        self.dashboard.loading = false;
        self.dashboard.rx = None;
        match result {
            Ok((user, my_prs, reviews)) => {
                self.dashboard.apply_success(user, my_prs, reviews);
                self.status = AppStatus::Ready;
                self.clamp_selection();
            }
            Err(status) => {
                self.status = status;
                self.dashboard.reset_after_error();
            }
        }
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        self.dashboard.rows(&self.status)
    }

    pub fn clamp_selection(&mut self) {
        self.dashboard.clamp_selection(&self.status);
    }

    pub fn next(&mut self) {
        self.dashboard.next(&self.status);
    }

    pub fn previous(&mut self) {
        self.dashboard.previous();
    }

    pub fn scroll_dashboard_down(&mut self) {
        self.dashboard.scroll_down();
    }

    pub fn scroll_dashboard_up(&mut self) {
        self.dashboard.scroll_up();
    }

    pub fn select_dashboard_row(&mut self, index: usize) {
        self.dashboard.select(index, &self.status);
    }

    pub fn toggle_selected_group(&mut self) {
        self.dashboard.toggle_selected_group(&self.status);
    }

    pub fn next_repo_page(&mut self) {
        self.dashboard.next_repo_page(&self.status);
    }

    pub fn previous_repo_page(&mut self) {
        self.dashboard.previous_repo_page(&self.status);
    }

    pub fn open_search(&mut self) {
        self.dashboard.open_search();
    }

    pub fn close_search(&mut self) {
        self.dashboard.close_search();
    }

    pub fn search_is_open(&self) -> bool {
        self.dashboard.search.is_some()
    }

    pub fn search_query(&self) -> Option<&str> {
        self.dashboard
            .search
            .as_ref()
            .map(|search| search.query.as_str())
    }

    pub fn selected_search_index(&self) -> Option<usize> {
        self.dashboard.search.as_ref().map(|search| search.selected)
    }

    pub fn push_search_char(&mut self, ch: char) {
        self.dashboard.push_search_char(ch);
    }

    pub fn pop_search_char(&mut self) {
        self.dashboard.pop_search_char();
    }

    pub fn next_search_match(&mut self) {
        self.dashboard.next_search_match();
    }

    pub fn previous_search_match(&mut self) {
        self.dashboard.previous_search_match();
    }

    pub fn search_matches(&self) -> Vec<DashboardSearchMatch> {
        self.dashboard.search_matches()
    }

    pub fn open_selected_search_match(&mut self) {
        let Some(pr) = self.dashboard.selected_search_match().map(|item| item.pr) else {
            return;
        };

        self.dashboard.close_search();
        self.open_detail_for_pr(pr);
    }

    pub fn open_selected_detail(&mut self) {
        let rows = self.rows();
        let Some(pr) = rows.get(self.dashboard.selected).and_then(Row::pr).cloned() else {
            return;
        };

        self.open_detail_for_pr(pr);
    }

    pub fn copy_selected_branch(&mut self) {
        let rows = self.rows();
        let Some(branch) = rows
            .get(self.dashboard.selected)
            .and_then(Row::pr)
            .map(|pr| pr.head_ref.clone())
            .filter(|branch| !branch.is_empty())
        else {
            return;
        };

        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(branch.clone()))
        {
            Ok(()) => {
                self.copy_notice = Some(CopyNotice {
                    message: format!("copied branch {branch}"),
                    expires_at: Instant::now() + COPY_NOTICE_DURATION,
                });
            }
            Err(error) => {
                self.status = AppStatus::Error(format!("failed to copy branch: {error}"));
            }
        }
    }

    pub fn status_message(&self) -> Option<&str> {
        match &self.status {
            AppStatus::Error(message) => Some(message),
            _ => None,
        }
    }

    pub fn copy_notice_message(&self) -> Option<&str> {
        self.copy_notice
            .as_ref()
            .map(|notice| notice.message.as_str())
    }

    fn open_detail_for_pr(&mut self, pr: PullRequest) {
        self.detail.open_placeholder(pr.clone());
        self.view = AppView::Detail;
        self.status = AppStatus::Ready;
        self.start_detail_load(pr.clone());
        self.start_discussion_load(pr);
    }

    pub fn poll_background(&mut self) -> bool {
        let mut changed = false;
        changed |= self.poll_dashboard_load();
        changed |= self.poll_detail_load();
        changed |= self.poll_discussion_load();
        changed |= self.poll_copy_notice();
        changed
    }

    fn poll_copy_notice(&mut self) -> bool {
        let Some(notice) = &self.copy_notice else {
            return false;
        };

        if Instant::now() < notice.expires_at {
            return false;
        }

        self.copy_notice = None;
        true
    }

    fn poll_dashboard_load(&mut self) -> bool {
        let Some(rx) = &self.dashboard.rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.apply_refresh_result(result);
        true
    }

    fn poll_detail_load(&mut self) -> bool {
        let Some(rx) = &self.detail.detail_rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.detail.apply_detail_result(result);
        true
    }

    fn poll_discussion_load(&mut self) -> bool {
        let Some(rx) = &self.detail.discussion_rx else {
            return false;
        };

        let Ok(result) = rx.try_recv() else {
            return false;
        };

        self.detail.apply_discussion_result(result);
        true
    }

    fn start_detail_load(&mut self, pr: PullRequest) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel();
        self.detail.detail_rx = Some(rx);
        self.detail.detail_status = DetailStatus::Loading;
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
        self.detail.discussion_rx = Some(rx);
        self.detail.discussion_status = DiscussionStatus::Loading;
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
        self.detail.clear();
    }

    pub fn scroll_active_down(&mut self) {
        self.detail.scroll_active_down();
    }

    pub fn scroll_active_up(&mut self) {
        self.detail.scroll_active_up();
    }

    pub fn toggle_detail_pane(&mut self) {
        self.detail.toggle_pane();
    }

    pub fn focus_detail_pane(&mut self, pane: DetailPane) {
        self.detail.focus_pane(pane);
    }

    pub fn next_discussion(&mut self) {
        self.detail.next_discussion();
    }

    pub fn previous_discussion(&mut self) {
        self.detail.previous_discussion();
    }

    pub fn selected_discussion_index(&self) -> usize {
        self.detail.selected_discussion_index()
    }

    pub fn open_selected_in_browser(&mut self) {
        let url = match self.view {
            AppView::Dashboard => self
                .rows()
                .get(self.dashboard.selected)
                .and_then(Row::pr_url)
                .map(str::to_owned),
            AppView::Detail => self
                .detail
                .current
                .as_ref()
                .map(|detail| detail.pr.url.clone()),
        };

        let Some(url) = url else {
            return;
        };

        if let Err(error) = webbrowser::open(&url) {
            self.status = AppStatus::Error(format!("failed to open browser: {error}"));
        }
    }

    pub fn show_dashboard_loading_screen(&self) -> bool {
        self.dashboard.show_loading_screen()
    }

    pub fn detail_is_loading(&self) -> bool {
        self.detail.detail_is_loading()
    }

    pub fn is_loading(&self) -> bool {
        self.dashboard.loading || self.detail_is_loading()
    }

    pub fn advance_loading_frame(&mut self) {
        if self.is_loading() {
            self.loading_frame = self.loading_frame.wrapping_add(1);
        }
    }

    pub fn dashboard_error_page(&self) -> Option<DashboardErrorPage> {
        status::dashboard_error_page(&self.status, self.dashboard.data.is_empty())
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
        self.dashboard.loading = false;
        self.dashboard.rx = None;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppView {
    Dashboard,
    Detail,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::GhStatus;
    use crate::model::{
        DiscussionItem, DiscussionKind, PullRequestDetail, Reviewer, ReviewerState,
    };
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

        assert_eq!(app.dashboard.current_user.as_deref(), Some("octocat"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(matches!(app.status, AppStatus::Ready));
        assert!(
            app.rows()
                .iter()
                .any(|row| matches!(row, Row::Pr { pr, .. } if pr.number == 1))
        );
        assert!(
            app.rows()
                .iter()
                .any(|row| matches!(row, Row::Pr { pr, .. } if pr.number == 2))
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

        app.dashboard.loading = true;
        assert!(app.show_dashboard_loading_screen());
        assert!(matches!(
            app.rows().last(),
            Some(Row::Message(message)) if !message.contains("Loading")
        ));

        app.dashboard.current_user = Some("octocat".to_owned());
        assert!(app.show_dashboard_loading_screen());

        app.dashboard.loading = false;
        app.refresh();
        app.dashboard.loading = true;
        assert!(!app.show_dashboard_loading_screen());
    }

    #[test]
    fn loading_frame_advances_only_while_loading() {
        let mut app = App::new(Box::new(TestSource::ok()));

        app.advance_loading_frame();
        assert_eq!(app.loading_frame, 0);

        app.dashboard.loading = true;
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
        assert_eq!(app.dashboard.selected, 1);
        app.toggle_selected_group();

        assert!(app.rows().len() < expanded_count);
        assert!(matches!(
            app.rows().get(1),
            Some(Row::Group { open: false, .. })
        ));
        app.previous();
        assert_eq!(app.dashboard.selected, 0);
    }

    #[test]
    fn repo_page_navigation_limits_visible_prs_per_repo() {
        let mut source = TestSource::ok();
        source.my_prs = Ok((1..=7).map(|number| pr("owner/repo", number)).collect());
        source.review_prs = Ok(Vec::new());
        let mut app = App::new(Box::new(source));
        app.refresh();

        let numbers: Vec<_> = app
            .rows()
            .into_iter()
            .filter_map(|row| row.pr().map(|pr| pr.number))
            .collect();
        assert_eq!(numbers, vec![1, 2, 3]);

        app.next();
        app.next_repo_page();
        let rows = app.rows();
        assert!(matches!(
            rows.get(app.dashboard.selected),
            Some(Row::Group {
                page: 2,
                page_count: 3,
                ..
            })
        ));
        let numbers: Vec<_> = rows
            .into_iter()
            .filter_map(|row| row.pr().map(|pr| pr.number))
            .collect();
        assert_eq!(numbers, vec![4, 5, 6]);

        app.previous_repo_page();
        let numbers: Vec<_> = app
            .rows()
            .into_iter()
            .filter_map(|row| row.pr().map(|pr| pr.number))
            .collect();
        assert_eq!(numbers, vec![1, 2, 3]);
    }

    #[test]
    fn search_open_close_and_query_edit_preserve_dashboard_selection() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();
        app.next();

        app.open_search();
        app.push_search_char('r');
        app.push_search_char('e');
        app.pop_search_char();
        app.close_search();

        assert!(app.dashboard.search.is_none());
        assert_eq!(app.dashboard.selected, 1);
    }

    #[test]
    fn search_returns_loaded_prs_even_when_group_is_collapsed() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();
        app.next();
        app.toggle_selected_group();

        assert!(
            !app.rows()
                .iter()
                .any(|row| matches!(row, Row::Pr { pr, .. } if pr.number == 1))
        );

        app.open_search();
        app.push_search_char('1');

        assert!(app.search_matches().iter().any(|item| item.pr.number == 1));
    }

    #[test]
    fn search_selection_clamps_and_opening_match_clears_search() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();
        app.open_search();
        app.next_search_match();
        app.push_search_char('2');

        assert_eq!(app.dashboard.search.as_ref().unwrap().selected, 0);
        app.open_selected_search_match();

        assert_eq!(app.view, AppView::Detail);
        assert!(app.dashboard.search.is_none());
        assert_eq!(app.detail.current.as_ref().unwrap().pr.number, 2);
    }

    #[test]
    fn opening_detail_uses_placeholder_then_background_results() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.refresh();
        app.next();
        app.next();
        app.open_selected_detail();

        assert_eq!(app.view, AppView::Detail);
        assert_eq!(app.detail.detail_status, DetailStatus::Loading);
        assert_eq!(app.detail.discussion_status, DiscussionStatus::Loading);
        assert_eq!(app.detail.current.as_ref().unwrap().pr.number, 1);

        poll_until_ready(&mut app);

        assert_eq!(app.detail.detail_status, DetailStatus::Ready);
        assert_eq!(app.detail.discussion_status, DiscussionStatus::Ready);
        let detail = app.detail.current.as_ref().unwrap();
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

        let pr = &app.detail.current.as_ref().unwrap().pr;
        assert_eq!(pr.review_decision.as_deref(), Some("APPROVED"));
        assert_eq!(pr.check_status.as_deref(), Some("passing"));
    }

    #[test]
    fn detail_pane_focus_controls_active_scroll_target() {
        let mut app = App::new(Box::new(TestSource::ok()));
        app.scroll_active_down();
        assert_eq!(app.detail.description_scroll, 1);
        assert_eq!(app.detail.discussion_scroll, 0);

        app.toggle_detail_pane();
        app.scroll_active_down();
        assert_eq!(app.detail.description_scroll, 1);
        assert_eq!(app.detail.discussion_scroll, 1);

        app.toggle_detail_pane();
        assert_eq!(app.detail.active_pane, DetailPane::Description);
        app.scroll_active_up();
        assert_eq!(app.detail.description_scroll, 0);
    }

    #[test]
    fn discussion_selection_wraps_and_resets_scroll() {
        let mut app = App::new(Box::new(TestSource::ok()));
        let mut detail = detail(pr("owner/repo", 1));
        detail.discussion = vec![
            discussion("alice", "2026-07-01T10:00:00Z"),
            discussion("bob", "2026-07-01T10:01:00Z"),
        ];
        app.detail.current = Some(detail);
        app.detail.discussion_scroll = 4;

        app.previous_discussion();
        assert_eq!(app.selected_discussion_index(), 1);
        assert_eq!(app.detail.discussion_scroll, 0);
        app.next_discussion();
        assert_eq!(app.selected_discussion_index(), 0);
    }

    fn poll_until_ready(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            app.poll_background();
            if app.detail.detail_status != DetailStatus::Loading
                && app.detail.discussion_status != DiscussionStatus::Loading
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
            head_ref: format!("feature-{number}"),
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
