use crate::app::{App, AppView};
use crate::github::MockErrorMode;
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};

pub(super) enum InputOutcome {
    Continue(bool),
    Quit,
}

pub(super) fn handle_event(event: Event, app: &mut App) -> Result<InputOutcome> {
    let key = match event {
        Event::Key(key) => key,
        Event::Resize(_, _) => return Ok(InputOutcome::Continue(true)),
        _ => return Ok(InputOutcome::Continue(false)),
    };

    if key.kind != KeyEventKind::Press {
        return Ok(InputOutcome::Continue(false));
    }

    let changed = match app.view {
        AppView::Dashboard => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(InputOutcome::Quit),
            KeyCode::Down | KeyCode::Char('j') => {
                app.next();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.previous();
                true
            }
            KeyCode::Char(' ') | KeyCode::Char('o') => {
                app.toggle_selected_group();
                true
            }
            KeyCode::Char('r') => {
                app.refresh_async();
                true
            }
            KeyCode::Char(key) if app.is_mock() && mock_error_mode_for_key(key).is_some() => {
                app.set_mock_error_mode(mock_error_mode_for_key(key).flatten());
                true
            }
            KeyCode::Char('b') => {
                app.open_selected_in_browser();
                true
            }
            KeyCode::Enter => {
                app.open_selected_detail();
                true
            }
            _ => false,
        },
        AppView::Detail => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.back_to_dashboard();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.scroll_active_down();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.scroll_active_up();
                true
            }
            KeyCode::Tab => {
                app.toggle_detail_pane();
                true
            }
            KeyCode::Char('b') => {
                app.open_selected_in_browser();
                true
            }
            KeyCode::Char('n') | KeyCode::Right => {
                app.next_discussion();
                true
            }
            KeyCode::Char('p') | KeyCode::Left => {
                app.previous_discussion();
                true
            }
            KeyCode::Char('r') => {
                app.open_selected_detail();
                true
            }
            _ => false,
        },
    };

    Ok(InputOutcome::Continue(changed))
}

/// Mock dashboard error controls: 0=ok, 1=GitHub down, 2=timeout, 3=generic error, 4=auth.
fn mock_error_mode_for_key(key: char) -> Option<Option<MockErrorMode>> {
    match key {
        '0' => Some(None),
        '1' => Some(Some(MockErrorMode::GitHubDown)),
        '2' => Some(Some(MockErrorMode::Timeout)),
        '3' => Some(Some(MockErrorMode::Generic)),
        '4' => Some(Some(MockErrorMode::Auth)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DetailPane, DetailStatus};
    use crate::github::{GhStatus, MockErrorMode, MockGhClient, PullRequestSource};
    use crate::model::{PullRequest, PullRequestDetail};
    use anyhow::Result;
    use crossterm::event::{KeyEvent, KeyEventState, KeyModifiers};

    #[derive(Clone)]
    struct EmptySource;

    impl PullRequestSource for EmptySource {
        fn clone_box(&self) -> Box<dyn PullRequestSource> {
            Box::new(self.clone())
        }

        fn status(&self) -> GhStatus {
            GhStatus::Ready {
                version: "test".to_owned(),
            }
        }

        fn current_user(&self) -> Result<String> {
            Ok("octocat".to_owned())
        }

        fn fetch_my_prs(&self, _login: &str) -> Result<Vec<PullRequest>> {
            Ok(Vec::new())
        }

        fn fetch_review_requests(&self, _login: &str) -> Result<Vec<PullRequest>> {
            Ok(Vec::new())
        }

        fn fetch_pr_detail(&self, pr: &PullRequest) -> Result<PullRequestDetail> {
            Ok(PullRequestDetail {
                pr: pr.clone(),
                body: String::new(),
                state: "OPEN".to_owned(),
                mergeable: None,
                head_ref: "feature".to_owned(),
                base_ref: "main".to_owned(),
                reviews: Vec::new(),
                discussion: Vec::new(),
            })
        }
    }

    #[test]
    fn resize_requests_redraw_and_key_release_is_ignored() {
        let mut app = App::new(Box::new(EmptySource));

        assert_continue_changed(
            handle_event(Event::Resize(120, 40), &mut app).unwrap(),
            true,
        );
        assert_continue_changed(
            handle_event(
                Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Release,
                    state: KeyEventState::NONE,
                }),
                &mut app,
            )
            .unwrap(),
            false,
        );
    }

    #[test]
    fn dashboard_keys_navigate_toggle_groups_and_quit() {
        let mut app = App::new(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('j'), &mut app), true);
        assert_eq!(app.selected, 1);

        assert_continue_changed(key(KeyCode::Char('o'), &mut app), true);
        assert!(matches!(
            app.rows().get(1),
            Some(crate::app::Row::Group { open: false, .. })
        ));

        assert_continue_changed(key(KeyCode::Char('k'), &mut app), true);
        assert_eq!(app.selected, 0);
        assert!(matches!(
            key(KeyCode::Char('q'), &mut app),
            InputOutcome::Quit
        ));
    }

    #[test]
    fn mock_error_keys_are_mock_only() {
        let mut mock_app = App::new(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::Char('1'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert_continue_changed(key(KeyCode::Char('0'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), None);

        let mut live_like_app = App::new(Box::new(EmptySource));
        assert_continue_changed(key(KeyCode::Char('1'), &mut live_like_app), false);
        assert_eq!(live_like_app.mock_error_mode(), None);
    }

    #[test]
    fn mock_error_keys_replace_active_mock_load() {
        let mut app = App::new(Box::new(MockGhClient::new()));
        app.dashboard_loading = true;

        assert_continue_changed(key(KeyCode::Char('1'), &mut app), true);

        assert_eq!(app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert!(app.dashboard_loading);
    }

    #[test]
    fn detail_keys_switch_panes_scroll_and_return_to_dashboard() {
        let mut app = App::new(Box::new(EmptySource));
        app.view = AppView::Detail;

        assert_continue_changed(key(KeyCode::Tab, &mut app), true);
        assert_eq!(app.detail.active_pane, DetailPane::Discussion);

        assert_continue_changed(key(KeyCode::Char('j'), &mut app), true);
        assert_eq!(app.detail.discussion_scroll, 1);

        assert_continue_changed(key(KeyCode::Tab, &mut app), true);
        assert_eq!(app.detail.active_pane, DetailPane::Description);

        assert_continue_changed(key(KeyCode::Char('D'), &mut app), false);

        assert_continue_changed(key(KeyCode::Esc, &mut app), true);
        assert_eq!(app.view, AppView::Dashboard);
        assert_eq!(app.detail.detail_status, DetailStatus::Idle);
    }

    fn key(code: KeyCode, app: &mut App) -> InputOutcome {
        handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), app).unwrap()
    }

    fn assert_continue_changed(outcome: InputOutcome, expected: bool) {
        match outcome {
            InputOutcome::Continue(changed) => assert_eq!(changed, expected),
            InputOutcome::Quit => panic!("expected continue outcome"),
        }
    }
}
