use super::rows::{DashboardSection, Row, group_key, group_names, page_count, push_groups};
use super::search::{DashboardSearchMatch, DashboardSearchState, search_matches};
use super::status::{AppStatus, github_outage_rows};
use crate::config::DashboardConfig;
use crate::model::{Dashboard, PullRequest};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::Receiver;

pub(super) type DashboardLoad = Result<(String, Vec<PullRequest>, Vec<PullRequest>), AppStatus>;
pub(super) type DashboardReceiver = Receiver<DashboardLoad>;

pub struct DashboardState {
    pub data: Dashboard,
    pub current_user: Option<String>,
    pub selected: usize,
    pub scroll: u16,
    active_section: DashboardSection,
    section_positions: [DashboardPosition; 2],
    pub(super) search: Option<DashboardSearchState>,
    pub loading: bool,
    collapsed_groups: BTreeSet<String>,
    repo_pages: BTreeMap<String, usize>,
    pub(super) rx: Option<DashboardReceiver>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DashboardPosition {
    selected: usize,
    scroll: u16,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            data: Dashboard::default(),
            current_user: None,
            selected: 0,
            scroll: 0,
            active_section: DashboardSection::MyPrs,
            section_positions: [DashboardPosition::default(); 2],
            search: None,
            loading: false,
            collapsed_groups: BTreeSet::new(),
            repo_pages: BTreeMap::new(),
            rx: None,
        }
    }

    pub fn rows(&self, status: &AppStatus, config: &DashboardConfig) -> Vec<Row<'_>> {
        match status {
            AppStatus::MissingGh if self.data.is_empty() => return vec![Row::Message("GitHub CLI `gh` was not found on PATH. Install it, authenticate it, then press r to retry.".to_owned())],
            AppStatus::Unauthenticated(_) if self.data.is_empty() => return vec![Row::Message("GitHub CLI is not authenticated. Run `gh auth login`, then press r to retry.".to_owned())],
            AppStatus::GitHubOutage(_) if self.data.is_empty() => return github_outage_rows(),
            AppStatus::Timeout(_) if self.data.is_empty() => return vec![
                Row::Message("GitHub is taking too long to answer.".to_owned()),
                Row::Message("The last gh command was stopped after 30s. Press r to retry.".to_owned()),
            ],
            AppStatus::Error(_) if self.data.is_empty() => return vec![Row::Message("Could not load pull requests. Press r to retry.".to_owned())],
            _ => {}
        }

        let mut rows = Vec::new();
        if config.separate_views {
            self.push_section_rows(&mut rows, self.active_section, config.prs_per_repo_page);
        } else {
            self.push_section_rows(&mut rows, DashboardSection::MyPrs, config.prs_per_repo_page);
            self.push_section_rows(
                &mut rows,
                DashboardSection::AwaitingReview,
                config.prs_per_repo_page,
            );
        }

        if self.data.is_empty() {
            rows.push(Row::Message(
                "No open PRs found. Press r to refresh.".to_owned(),
            ));
        }

        rows
    }

    fn push_section_rows<'a>(
        &'a self,
        rows: &mut Vec<Row<'a>>,
        section: DashboardSection,
        page_size: usize,
    ) {
        let groups = match section {
            DashboardSection::MyPrs => &self.data.my_prs,
            DashboardSection::AwaitingReview => &self.data.awaiting_review,
        };
        rows.push(Row::Section(section.title()));
        push_groups(
            rows,
            section,
            groups,
            &self.collapsed_groups,
            &self.repo_pages,
            page_size,
        );
    }

    pub fn active_section(&self) -> DashboardSection {
        self.active_section
    }

    pub fn show_section(
        &mut self,
        section: DashboardSection,
        status: &AppStatus,
        config: &DashboardConfig,
    ) -> bool {
        if !config.separate_views || self.active_section == section {
            return false;
        }

        self.section_positions[self.active_section.position_index()] = DashboardPosition {
            selected: self.selected,
            scroll: self.scroll,
        };
        self.active_section = section;
        let position = self.section_positions[section.position_index()];
        self.selected = position.selected;
        self.scroll = position.scroll;
        self.clamp_selection(status, config);
        true
    }

    pub fn cycle_section(&mut self, status: &AppStatus, config: &DashboardConfig) -> bool {
        let section = match self.active_section {
            DashboardSection::MyPrs => DashboardSection::AwaitingReview,
            DashboardSection::AwaitingReview => DashboardSection::MyPrs,
        };
        self.show_section(section, status, config)
    }

    pub fn section_pr_count(&self, section: DashboardSection) -> usize {
        let groups = match section {
            DashboardSection::MyPrs => &self.data.my_prs,
            DashboardSection::AwaitingReview => &self.data.awaiting_review,
        };
        groups.iter().map(|group| group.prs.len()).sum()
    }

    pub fn apply_success(
        &mut self,
        user: String,
        my_prs: Vec<PullRequest>,
        reviews: Vec<PullRequest>,
        config: &DashboardConfig,
    ) {
        self.current_user = Some(user);
        self.data = Dashboard::from_prs(my_prs, reviews);
        self.clamp_search_selection();
        let valid_groups = group_names(&self.data);
        self.collapsed_groups = self
            .collapsed_groups
            .intersection(&valid_groups)
            .cloned()
            .collect();
        self.clamp_repo_pages(config.prs_per_repo_page);
    }

    pub fn reset_after_error(&mut self) {
        self.data = Dashboard::default();
        self.selected = 0;
        self.close_search();
    }

    pub fn clamp_selection(&mut self, status: &AppStatus, config: &DashboardConfig) {
        self.selected = self
            .selected
            .min(self.rows(status, config).len().saturating_sub(1));
    }

    pub fn next(&mut self, status: &AppStatus, config: &DashboardConfig) {
        let len = self.rows(status, config).len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn select(&mut self, index: usize, status: &AppStatus, config: &DashboardConfig) {
        self.selected = index;
        self.clamp_selection(status, config);
    }

    pub fn toggle_selected_group(&mut self, status: &AppStatus, config: &DashboardConfig) {
        let rows = self.rows(status, config);
        let Some(repo) = rows.get(self.selected).and_then(Row::group_key) else {
            return;
        };

        if !self.collapsed_groups.insert(repo.clone()) {
            self.collapsed_groups.remove(&repo);
        }
        self.clamp_selection(status, config);
    }

    pub fn next_repo_page(&mut self, status: &AppStatus, config: &DashboardConfig) {
        self.change_repo_page(status, config, 1);
    }

    pub fn previous_repo_page(&mut self, status: &AppStatus, config: &DashboardConfig) {
        self.change_repo_page(status, config, -1);
    }

    fn change_repo_page(&mut self, status: &AppStatus, config: &DashboardConfig, delta: i8) {
        let rows = self.rows(status, config);
        let Some(key) = selected_group_key(&rows, self.selected) else {
            return;
        };
        let Some(max_page) = self.max_page_for_group(&key, config.prs_per_repo_page) else {
            return;
        };
        let page = self.repo_pages.get(&key).copied().unwrap_or_default();
        let next = if delta.is_positive() {
            (page + 1).min(max_page)
        } else {
            page.saturating_sub(1)
        };

        self.repo_pages.insert(key.clone(), next);
        if let Some(index) = group_row_index(&self.rows(status, config), &key) {
            self.selected = index;
        }
    }

    fn clamp_repo_pages(&mut self, page_size: usize) {
        let max_pages: BTreeMap<_, _> = self
            .data
            .my_prs
            .iter()
            .map(|group| {
                (
                    group_key(DashboardSection::MyPrs, &group.repo),
                    page_count(group.prs.len(), page_size) - 1,
                )
            })
            .chain(self.data.awaiting_review.iter().map(|group| {
                (
                    group_key(DashboardSection::AwaitingReview, &group.repo),
                    page_count(group.prs.len(), page_size) - 1,
                )
            }))
            .collect();

        self.repo_pages.retain(|key, page| {
            let Some(max_page) = max_pages.get(key) else {
                return false;
            };
            *page = (*page).min(*max_page);
            true
        });
    }

    fn max_page_for_group(&self, key: &str, page_size: usize) -> Option<usize> {
        self.data
            .my_prs
            .iter()
            .map(|group| (DashboardSection::MyPrs, group))
            .chain(
                self.data
                    .awaiting_review
                    .iter()
                    .map(|group| (DashboardSection::AwaitingReview, group)),
            )
            .find(|(section, group)| group_key(*section, &group.repo) == key)
            .map(|(_, group)| page_count(group.prs.len(), page_size) - 1)
    }

    pub fn show_loading_screen(&self) -> bool {
        self.loading
    }

    pub fn open_search(&mut self) {
        self.search = Some(DashboardSearchState::default());
    }

    pub fn close_search(&mut self) {
        self.search = None;
    }

    pub fn push_search_char(&mut self, ch: char) {
        let Some(search) = &mut self.search else {
            return;
        };

        search.query.push(ch);
        search.selected = 0;
    }

    pub fn pop_search_char(&mut self) {
        let Some(search) = &mut self.search else {
            return;
        };

        search.query.pop();
        search.selected = 0;
    }

    pub fn next_search_match(&mut self) {
        let len = self.search_matches().len();
        let Some(search) = &mut self.search else {
            return;
        };

        if len > 0 {
            search.selected = (search.selected + 1).min(len - 1);
        }
    }

    pub fn previous_search_match(&mut self) {
        let Some(search) = &mut self.search else {
            return;
        };

        search.selected = search.selected.saturating_sub(1);
    }

    pub fn search_matches(&self) -> Vec<DashboardSearchMatch> {
        let Some(search) = &self.search else {
            return Vec::new();
        };

        search_matches(&self.data, &search.query)
    }

    pub fn selected_search_match(&mut self) -> Option<DashboardSearchMatch> {
        self.clamp_search_selection();
        let selected = self.search.as_ref()?.selected;
        self.search_matches().get(selected).cloned()
    }

    fn clamp_search_selection(&mut self) {
        let len = self.search_matches().len();
        let Some(search) = &mut self.search else {
            return;
        };

        search.selected = search.selected.min(len.saturating_sub(1));
    }
}

fn selected_group_key(rows: &[Row<'_>], selected: usize) -> Option<String> {
    rows.get(selected).and_then(Row::group_key).or_else(|| {
        rows.iter()
            .take(selected + 1)
            .rev()
            .find_map(Row::group_key)
    })
}

fn group_row_index(rows: &[Row<'_>], key: &str) -> Option<usize> {
    rows.iter()
        .position(|row| row.group_key().as_deref() == Some(key))
}
