use crate::model::{DiscussionItem, DiscussionKind, PullRequest, PullRequestDetail};
use std::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct DetailState {
    pub current: Option<PullRequestDetail>,
    pub description_scroll: u16,
    pub discussion_selected: usize,
    pub discussion_scroll: u16,
    pub active_pane: DetailPane,
    pub detail_status: DetailStatus,
    pub discussion_status: DiscussionStatus,
    pub(super) detail_rx: Option<Receiver<Result<PullRequestDetail, String>>>,
    pub(super) discussion_rx: Option<Receiver<Result<Vec<DiscussionItem>, String>>>,
}

impl DetailState {
    pub fn new() -> Self {
        Self {
            current: None,
            description_scroll: 0,
            discussion_selected: 0,
            discussion_scroll: 0,
            active_pane: DetailPane::Description,
            detail_status: DetailStatus::Idle,
            discussion_status: DiscussionStatus::Idle,
            detail_rx: None,
            discussion_rx: None,
        }
    }

    pub fn open_placeholder(&mut self, pr: PullRequest) {
        self.current = Some(placeholder_detail(pr));
        self.description_scroll = 0;
        self.discussion_selected = 0;
        self.discussion_scroll = 0;
        self.active_pane = DetailPane::Description;
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.description_scroll = 0;
        self.discussion_selected = 0;
        self.discussion_scroll = 0;
        self.active_pane = DetailPane::Description;
        self.detail_status = DetailStatus::Idle;
        self.discussion_status = DiscussionStatus::Idle;
        self.detail_rx = None;
        self.discussion_rx = None;
    }

    pub fn detail_is_loading(&self) -> bool {
        self.detail_status == DetailStatus::Loading
            || self.discussion_status == DiscussionStatus::Loading
    }

    pub fn scroll_active_down(&mut self) {
        match self.active_pane {
            DetailPane::Description => {
                self.description_scroll = self.description_scroll.saturating_add(1);
            }
            DetailPane::Discussion => {
                self.discussion_scroll = self.discussion_scroll.saturating_add(1);
            }
        }
    }

    pub fn scroll_active_up(&mut self) {
        match self.active_pane {
            DetailPane::Description => {
                self.description_scroll = self.description_scroll.saturating_sub(1);
            }
            DetailPane::Discussion => {
                self.discussion_scroll = self.discussion_scroll.saturating_sub(1);
            }
        }
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            DetailPane::Description => DetailPane::Discussion,
            DetailPane::Discussion => DetailPane::Description,
        };
    }

    pub fn focus_pane(&mut self, pane: DetailPane) {
        self.active_pane = pane;
    }

    pub fn next_discussion(&mut self) {
        let Some(detail) = &self.current else {
            return;
        };
        if !detail.discussion.is_empty() {
            self.discussion_selected = (self.discussion_selected + 1) % detail.discussion.len();
            self.discussion_scroll = 0;
        }
    }

    pub fn previous_discussion(&mut self) {
        let Some(detail) = &self.current else {
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
        self.current
            .as_ref()
            .filter(|detail| !detail.discussion.is_empty())
            .map(|detail| self.discussion_selected.min(detail.discussion.len() - 1))
            .unwrap_or(0)
    }

    pub(super) fn apply_detail_result(&mut self, result: Result<PullRequestDetail, String>) {
        self.detail_rx = None;
        match result {
            Ok(mut loaded_detail) => {
                if let Some(current_detail) = &mut self.current {
                    let mut existing_review_threads: Vec<_> = current_detail
                        .discussion
                        .iter()
                        .filter(|item| matches!(item.kind, DiscussionKind::ReviewThread { .. }))
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
                    sort_discussion(&mut loaded_detail.discussion);
                    *current_detail = loaded_detail;
                }
                self.detail_status = DetailStatus::Ready;
            }
            Err(error) => {
                self.detail_status = DetailStatus::Error(error);
            }
        }
    }

    pub(super) fn apply_discussion_result(&mut self, result: Result<Vec<DiscussionItem>, String>) {
        self.discussion_rx = None;
        match result {
            Ok(mut discussion) => {
                if let Some(detail) = &mut self.current {
                    detail.discussion.append(&mut discussion);
                    sort_discussion(&mut detail.discussion);
                }
                self.discussion_status = DiscussionStatus::Ready;
            }
            Err(error) => {
                self.discussion_status = DiscussionStatus::Error(error);
            }
        }
    }
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

fn placeholder_detail(pr: PullRequest) -> PullRequestDetail {
    let head_ref = pr.head_ref.clone();
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
        head_ref,
        base_ref: String::new(),
        reviews: Vec::new(),
        discussion: Vec::new(),
    }
}

fn sort_discussion(discussion: &mut [DiscussionItem]) {
    discussion.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.url.cmp(&right.url))
    });
}
