use crate::github::PullRequestSource;
use crate::model::{Dashboard, PullRequest, PullRequestDetail, RepoGroup, repo_names};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct App {
    client: Box<dyn PullRequestSource>,
    dashboard: Dashboard,
    collapsed_repos: BTreeSet<String>,
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
    detail_rx: Option<Receiver<Result<PullRequestDetail, String>>>,
    discussion_rx: Option<Receiver<Result<Vec<crate::model::DiscussionItem>, String>>>,
}

impl App {
    pub fn new(client: Box<dyn PullRequestSource>) -> Self {
        Self {
            client,
            dashboard: Dashboard::default(),
            collapsed_repos: BTreeSet::new(),
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
            detail_rx: None,
            discussion_rx: None,
        }
    }

    pub fn refresh(&mut self) {
        let result = self.refresh_dashboard();

        match result {
            Ok((user, my_prs, reviews)) => {
                self.current_user = Some(user);
                self.dashboard = Dashboard::from_prs(my_prs, reviews);
                self.collapsed_repos = self
                    .collapsed_repos
                    .intersection(&repo_names(&self.dashboard))
                    .cloned()
                    .collect();
                self.status = AppStatus::Ready;
                self.clamp_selection();
            }
            Err(error) => {
                self.status = classify_refresh_error(error.to_string());
                self.dashboard = Dashboard::default();
            }
        }
    }

    fn refresh_dashboard(
        &mut self,
    ) -> anyhow::Result<(String, Vec<PullRequest>, Vec<PullRequest>)> {
        let user = match &self.current_user {
            Some(user) => user.clone(),
            None => self.client.current_user()?,
        };

        let my_client = self.client.clone();
        let review_client = self.client.clone();
        let review_user = user.clone();
        let my_handle = thread::spawn(move || my_client.fetch_my_prs());
        let review_handle =
            thread::spawn(move || review_client.fetch_review_requests(&review_user));

        let my_prs = my_handle
            .join()
            .map_err(|_| anyhow::anyhow!("authored PR fetch thread panicked"))??;
        let reviews = review_handle
            .join()
            .map_err(|_| anyhow::anyhow!("review-request fetch thread panicked"))??;

        Ok((user, my_prs, reviews))
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        match &self.status {
            AppStatus::MissingGh => return vec![Row::Message("GitHub CLI `gh` was not found on PATH. Install it, authenticate it, then press r to retry.".to_owned())],
            AppStatus::Unauthenticated(message) => return vec![Row::Message(format!("GitHub CLI is not authenticated. Run `gh auth login`, then press r to retry. {message}"))],
            AppStatus::Error(message) if self.dashboard.is_empty() => return vec![Row::Message(format!("Could not load pull requests. Press r to retry. {message}"))],
            _ => {}
        }

        let mut rows = Vec::new();
        rows.push(Row::Section("My PRs"));
        push_groups(&mut rows, &self.dashboard.my_prs, &self.collapsed_repos);
        rows.push(Row::Section("Awaiting Review"));
        push_groups(
            &mut rows,
            &self.dashboard.awaiting_review,
            &self.collapsed_repos,
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
        let Some(repo) = rows
            .get(self.selected)
            .and_then(Row::repo_name)
            .map(str::to_owned)
        else {
            return;
        };

        if !self.collapsed_repos.insert(repo.clone()) {
            self.collapsed_repos.remove(&repo);
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

    pub fn poll_background(&mut self) {
        self.poll_detail_load();
        self.poll_discussion_load();
    }

    fn poll_detail_load(&mut self) {
        let Some(rx) = &self.detail_rx else {
            return;
        };

        let Ok(result) = rx.try_recv() else {
            return;
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
    }

    fn poll_discussion_load(&mut self) {
        let Some(rx) = &self.discussion_rx else {
            return;
        };

        let Ok(result) = rx.try_recv() else {
            return;
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
    }

    fn start_detail_load(&mut self, pr: PullRequest) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel();
        self.detail_rx = Some(rx);
        self.detail_status = DetailStatus::Loading;

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

    pub fn focus_description(&mut self) {
        self.active_detail_pane = DetailPane::Description;
    }

    pub fn focus_discussion(&mut self) {
        self.active_detail_pane = DetailPane::Discussion;
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
}

fn classify_refresh_error(message: String) -> AppStatus {
    if message.contains("executable file not found")
        || message.contains("No such file or directory")
        || message.contains("failed to run gh")
    {
        AppStatus::MissingGh
    } else if message.contains("gh auth login")
        || message.contains("not logged")
        || message.contains("authentication")
        || message.contains("HTTP 401")
    {
        AppStatus::Unauthenticated(message)
    } else {
        AppStatus::Error(message)
    }
}

fn placeholder_detail(pr: PullRequest) -> PullRequestDetail {
    let state = if pr.state.is_empty() {
        "loading".to_owned()
    } else {
        pr.state.clone()
    };

    PullRequestDetail {
        pr,
        body: String::new(),
        state,
        mergeable: None,
        head_ref: "loading".to_owned(),
        base_ref: "loading".to_owned(),
        reviews: Vec::new(),
        discussion: Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppView {
    Dashboard,
    Detail,
}

#[derive(Clone, Debug)]
pub enum AppStatus {
    Ready,
    MissingGh,
    Unauthenticated(String),
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

#[derive(Clone, Debug)]
pub enum Row<'a> {
    Section(&'static str),
    Group {
        repo: &'a str,
        count: usize,
        open: bool,
    },
    Pr(&'a PullRequest),
    Message(String),
}

impl Row<'_> {
    fn repo_name(&self) -> Option<&str> {
        match self {
            Row::Group { repo, .. } => Some(repo),
            _ => None,
        }
    }

    fn pr_url(&self) -> Option<&str> {
        match self {
            Row::Pr(pr) => Some(&pr.url),
            _ => None,
        }
    }

    fn pr(&self) -> Option<&PullRequest> {
        match self {
            Row::Pr(pr) => Some(pr),
            _ => None,
        }
    }
}

fn push_groups<'a>(rows: &mut Vec<Row<'a>>, groups: &'a [RepoGroup], collapsed: &BTreeSet<String>) {
    if groups.is_empty() {
        rows.push(Row::Message("  none".to_owned()));
        return;
    }

    for group in groups {
        let open = !collapsed.contains(&group.repo);
        rows.push(Row::Group {
            repo: &group.repo,
            count: group.prs.len(),
            open,
        });

        if open {
            rows.extend(group.prs.iter().map(Row::Pr));
        }
    }
}
