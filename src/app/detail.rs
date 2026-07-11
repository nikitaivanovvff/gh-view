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
                    if loaded_detail.pr.check_status.is_none() {
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
                    detail
                        .discussion
                        .retain(|item| !matches!(item.kind, DiscussionKind::ReviewThread { .. }));
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discussion_result_replaces_existing_review_threads() {
        let mut state = detail_state();
        state.apply_discussion_result(Ok(vec![discussion(
            DiscussionKind::ReviewThread { resolved: true },
            "new-thread",
        )]));

        let discussion = &state.current.as_ref().unwrap().discussion;
        assert_eq!(discussion.len(), 2);
        assert_eq!(discussion[0].body, "issue-comment");
        assert_eq!(discussion[1].body, "new-thread");
    }

    #[test]
    fn detail_and_discussion_results_merge_in_either_completion_order() {
        for discussion_first in [false, true] {
            let mut state = detail_state();
            let loaded = detail(vec![discussion(
                DiscussionKind::IssueComment,
                "loaded-comment",
            )]);
            let threads = vec![discussion(
                DiscussionKind::ReviewThread { resolved: false },
                "fresh-thread",
            )];

            if discussion_first {
                state.apply_discussion_result(Ok(threads));
                state.apply_detail_result(Ok(loaded));
            } else {
                state.apply_detail_result(Ok(loaded));
                state.apply_discussion_result(Ok(threads));
            }

            let discussion = &state.current.as_ref().unwrap().discussion;
            assert_eq!(discussion.len(), 2);
            assert_eq!(discussion[0].body, "loaded-comment");
            assert_eq!(discussion[1].body, "fresh-thread");
        }
    }

    fn detail_state() -> DetailState {
        let mut state = DetailState::new();
        state.current = Some(detail(vec![
            discussion(DiscussionKind::IssueComment, "issue-comment"),
            discussion(
                DiscussionKind::ReviewThread { resolved: false },
                "old-thread",
            ),
        ]));
        state
    }

    fn detail(discussion: Vec<DiscussionItem>) -> PullRequestDetail {
        PullRequestDetail {
            pr: PullRequest {
                repo: "owner/repo".to_owned(),
                number: 1,
                title: "Title".to_owned(),
                author: "author".to_owned(),
                head_ref: "branch".to_owned(),
                url: "https://example.test/pr/1".to_owned(),
                updated_at: "2026-01-01T00:00:00Z".to_owned(),
                state: "OPEN".to_owned(),
                is_draft: false,
                review_decision: None,
                check_status: None,
                reviewers: Vec::new(),
                review_requested: Vec::new(),
            },
            body: String::new(),
            state: "OPEN".to_owned(),
            mergeable: None,
            head_ref: "branch".to_owned(),
            base_ref: "main".to_owned(),
            reviews: Vec::new(),
            discussion,
        }
    }

    fn discussion(kind: DiscussionKind, body: &str) -> DiscussionItem {
        DiscussionItem {
            kind,
            author: "author".to_owned(),
            body: body.to_owned(),
            created_at: match body {
                "issue-comment" | "loaded-comment" => "2026-01-01T00:00:00Z",
                _ => "2026-01-02T00:00:00Z",
            }
            .to_owned(),
            url: String::new(),
            replies: Vec::new(),
            code_context: None,
        }
    }
}
