use super::layout::{MouseLayout, MouseTarget};
use super::theme;
use crate::app::{App, AppView, DashboardSection};
use crate::github::MockErrorMode;
use anyhow::Result;
use crossterm::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

pub(super) enum InputOutcome {
    Continue(bool),
    Quit,
}

pub(super) fn handle_event(
    event: Event,
    app: &mut App,
    mouse_layout: &MouseLayout,
) -> Result<InputOutcome> {
    let key = match event {
        Event::Key(key) => key,
        Event::Mouse(mouse) => {
            return Ok(InputOutcome::Continue(handle_mouse(
                mouse,
                app,
                mouse_layout,
            )));
        }
        Event::Resize(_, _) => return Ok(InputOutcome::Continue(true)),
        _ => return Ok(InputOutcome::Continue(false)),
    };

    if key.kind != KeyEventKind::Press {
        return Ok(InputOutcome::Continue(false));
    }

    if app.theme_picker_is_open() {
        let changed = match key.code {
            KeyCode::Enter => {
                let theme = theme::theme_key(app.selected_theme_index())
                    .expect("selected theme index should be valid");
                app.save_theme_picker(theme)?;
                true
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                app.cancel_theme_picker();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.next_theme(theme::theme_count());
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.previous_theme(theme::theme_count());
                true
            }
            _ => false,
        };
        return Ok(InputOutcome::Continue(changed));
    }

    if app.mock_debug_is_open() {
        let changed = match key.code {
            KeyCode::F(1) | KeyCode::Esc | KeyCode::Char('q') => {
                app.close_mock_debug();
                true
            }
            KeyCode::Char(key) => {
                let Some(mode) = mock_error_mode_for_key(key) else {
                    return Ok(InputOutcome::Continue(false));
                };
                app.close_mock_debug();
                app.set_mock_error_mode(mode);
                true
            }
            _ => false,
        };
        return Ok(InputOutcome::Continue(changed));
    }

    if app.help_is_open() {
        let changed = match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                app.close_help();
                true
            }
            _ => false,
        };
        return Ok(InputOutcome::Continue(changed));
    }

    let changed = match app.view {
        AppView::Dashboard if app.search_is_open() => match key.code {
            KeyCode::Esc => {
                app.close_search();
                true
            }
            KeyCode::Enter => {
                app.open_selected_search_match();
                true
            }
            KeyCode::Backspace => {
                app.pop_search_char();
                true
            }
            KeyCode::Down => {
                app.next_search_match();
                true
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.next_search_match();
                true
            }
            KeyCode::Up => {
                app.previous_search_match();
                true
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.previous_search_match();
                true
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.push_search_char(ch);
                true
            }
            _ => false,
        },
        AppView::Dashboard => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(InputOutcome::Quit),
            KeyCode::F(1) => app.toggle_mock_debug(),
            KeyCode::Char('?') => {
                app.open_help();
                true
            }
            KeyCode::Char('1') => app.show_dashboard_section(DashboardSection::MyPrs),
            KeyCode::Char('2') => app.show_dashboard_section(DashboardSection::AwaitingReview),
            KeyCode::Tab => app.cycle_dashboard_section(),
            KeyCode::Char('f') => app.cycle_review_scope(),
            KeyCode::Char('/') => app.open_search(),
            KeyCode::Char('t') => {
                app.open_theme_picker();
                true
            }
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
            KeyCode::Char('n') | KeyCode::Right => {
                app.next_repo_page();
                true
            }
            KeyCode::Char('p') | KeyCode::Left => {
                app.previous_repo_page();
                true
            }
            KeyCode::Char('r') => {
                app.refresh_async();
                true
            }
            KeyCode::Char('b') => {
                app.open_selected_in_browser();
                true
            }
            KeyCode::Char('c') => {
                app.copy_selected_branch();
                true
            }
            KeyCode::Enter => {
                app.open_selected_detail();
                true
            }
            _ => false,
        },
        AppView::Detail => match key.code {
            KeyCode::Char('?') => {
                app.open_help();
                true
            }
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
                app.refresh_detail_async();
                true
            }
            _ => false,
        },
    };

    Ok(InputOutcome::Continue(changed))
}

fn handle_mouse(mouse: MouseEvent, app: &mut App, mouse_layout: &MouseLayout) -> bool {
    let target = mouse_layout.target_at(Position::new(mouse.column, mouse.row));
    if app.theme_picker_is_open() {
        return handle_theme_picker_mouse(mouse, app, target);
    }
    if app.mock_debug_is_open() {
        return false;
    }
    if app.help_is_open() {
        return false;
    }

    if app.search_is_open() {
        return match mouse.kind {
            MouseEventKind::ScrollDown => {
                app.next_search_match();
                true
            }
            MouseEventKind::ScrollUp => {
                app.previous_search_match();
                true
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(MouseTarget::SearchMatch(index)) = target else {
                    return false;
                };
                app.select_search_match(index);
                app.open_selected_search_match();
                true
            }
            _ => false,
        };
    }

    match app.view {
        AppView::Dashboard => handle_dashboard_mouse(mouse, app, target),
        AppView::Detail => handle_detail_mouse(mouse, app, target),
    }
}

fn handle_theme_picker_mouse(
    mouse: MouseEvent,
    app: &mut App,
    target: Option<MouseTarget>,
) -> bool {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.next_theme(theme::theme_count());
            true
        }
        MouseEventKind::ScrollUp => {
            app.previous_theme(theme::theme_count());
            true
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let Some(MouseTarget::Theme(index)) = target else {
                return false;
            };
            app.select_theme(index, theme::theme_count())
        }
        _ => false,
    }
}

fn handle_dashboard_mouse(mouse: MouseEvent, app: &mut App, target: Option<MouseTarget>) -> bool {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.scroll_dashboard_down();
            true
        }
        MouseEventKind::ScrollUp => {
            app.scroll_dashboard_up();
            true
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if target == Some(MouseTarget::DashboardRetry) {
                app.refresh_async();
                return true;
            }
            if let Some(MouseTarget::DashboardSection(section)) = target {
                return app.show_dashboard_section(section);
            }
            if let Some(MouseTarget::ReviewScope(scope)) = target {
                return app.set_review_scope(scope);
            }

            let rows = app.rows();
            let Some(MouseTarget::DashboardRow(index)) = target else {
                return false;
            };
            let opens_selected_pr = index == app.dashboard.selected
                && matches!(rows.get(index), Some(crate::app::Row::Pr(_)));
            let toggles_selected_group = index == app.dashboard.selected
                && matches!(rows.get(index), Some(crate::app::Row::Group { .. }));
            app.select_dashboard_row(index);
            if opens_selected_pr {
                app.open_selected_detail();
            } else if toggles_selected_group {
                app.toggle_selected_group();
            }
            true
        }
        _ => false,
    }
}

fn handle_detail_mouse(mouse: MouseEvent, app: &mut App, target: Option<MouseTarget>) -> bool {
    let Some(MouseTarget::DetailPane(pane)) = target else {
        return false;
    };

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.focus_detail_pane(pane);
            app.scroll_active_down();
            true
        }
        MouseEventKind::ScrollUp => {
            app.focus_detail_pane(pane);
            app.scroll_active_up();
            true
        }
        MouseEventKind::Down(MouseButton::Left) => {
            app.focus_detail_pane(pane);
            true
        }
        _ => false,
    }
}

fn mock_error_mode_for_key(key: char) -> Option<Option<MockErrorMode>> {
    match key {
        '0' => Some(None),
        '5' => Some(Some(MockErrorMode::GitHubDown)),
        '6' => Some(Some(MockErrorMode::Timeout)),
        '7' => Some(Some(MockErrorMode::Generic)),
        '8' => Some(Some(MockErrorMode::Auth)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppStatus, DetailPane, DetailStatus};
    use crate::github::{GhStatus, MockErrorMode, MockGhClient, PullRequestSource};
    use crate::model::{PullRequest, PullRequestDetail};
    use crate::ui::render::render;
    use anyhow::Result;
    use crossterm::event::{KeyEvent, KeyEventState, KeyModifiers, MouseEvent};
    use ratatui::{Terminal, backend::TestBackend};

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
        let mut app = App::with_default_config(Box::new(EmptySource));

        assert_continue_changed(
            handle_event(Event::Resize(120, 40), &mut app, &MouseLayout::default()).unwrap(),
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
                &MouseLayout::default(),
            )
            .unwrap(),
            false,
        );
    }

    #[test]
    fn dashboard_keys_navigate_toggle_groups_and_quit() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('o'), &mut app), true);
        assert!(matches!(
            app.rows().first(),
            Some(crate::app::Row::Group { open: false, .. })
        ));
        assert_continue_changed(key(KeyCode::Char('o'), &mut app), true);
        assert_continue_changed(key(KeyCode::Char('j'), &mut app), true);
        assert_eq!(app.dashboard.selected, 1);

        assert_continue_changed(key(KeyCode::Char('k'), &mut app), true);
        assert_eq!(app.dashboard.selected, 0);
        assert!(matches!(
            key(KeyCode::Char('q'), &mut app),
            InputOutcome::Quit
        ));
    }

    #[test]
    fn question_mark_opens_and_closes_contextual_help() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::Char('?'), &mut app), true);
        assert!(app.help_is_open());
        assert_continue_changed(key(KeyCode::Char('j'), &mut app), false);
        assert_continue_changed(key(KeyCode::Char('?'), &mut app), true);
        assert!(!app.help_is_open());

        app.view = AppView::Detail;
        assert_continue_changed(key(KeyCode::Char('?'), &mut app), true);
        assert!(app.help_is_open());
        assert_continue_changed(key(KeyCode::Esc, &mut app), true);
        assert!(!app.help_is_open());
    }

    #[test]
    fn dashboard_view_keys_switch_sections() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('2'), &mut app), true);
        assert_eq!(
            app.dashboard.active_section(),
            DashboardSection::AwaitingReview
        );
        assert_eq!(app.mock_error_mode(), None);

        assert_continue_changed(key(KeyCode::F(1), &mut app), true);
        assert_continue_changed(key(KeyCode::Char('5'), &mut app), true);
        assert_eq!(app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert_continue_changed(key(KeyCode::F(1), &mut app), true);
        assert_continue_changed(key(KeyCode::Char('0'), &mut app), true);
        assert_eq!(app.mock_error_mode(), None);

        assert_continue_changed(key(KeyCode::Char('1'), &mut app), true);
        assert_eq!(app.dashboard.active_section(), DashboardSection::MyPrs);

        assert_continue_changed(key(KeyCode::Tab, &mut app), true);
        assert_eq!(
            app.dashboard.active_section(),
            DashboardSection::AwaitingReview
        );

        assert_continue_changed(key(KeyCode::Char('f'), &mut app), true);
        assert_eq!(
            app.dashboard.review_scope(),
            crate::app::ReviewScope::Direct
        );
    }

    #[test]
    fn search_captures_dashboard_view_keys() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.open_search();

        assert_continue_changed(key(KeyCode::Char('2'), &mut app), true);

        assert_eq!(app.search_query(), Some("2"));
        assert_eq!(app.dashboard.active_section(), DashboardSection::MyPrs);
    }

    #[test]
    fn search_does_not_open_behind_startup_screens() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.dashboard.loading = true;

        assert_continue_changed(key(KeyCode::Char('/'), &mut app), false);
        assert!(!app.search_is_open());

        app.dashboard.loading = false;
        app.status = AppStatus::Error("failed".to_owned());
        assert_continue_changed(key(KeyCode::Char('/'), &mut app), false);
        assert!(!app.search_is_open());
    }

    #[test]
    fn dashboard_mouse_selects_first_and_activates_selected_pr() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(mouse(MouseEventKind::ScrollDown, 0, 0, &mut app), true);
        assert_eq!(app.dashboard.selected, 0);
        assert_eq!(app.dashboard.scroll, 1);

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 5, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Pr(_))
        ));

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 5, &mut app),
            true,
        );
        assert_eq!(app.view, AppView::Detail);
    }

    #[test]
    fn dashboard_tab_clicks_switch_views() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 20, 2, &mut app),
            true,
        );
        assert_eq!(
            app.dashboard.active_section(),
            DashboardSection::AwaitingReview
        );

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 2, &mut app),
            true,
        );
        assert_eq!(app.dashboard.active_section(), DashboardSection::MyPrs);

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 2, 2, &mut app),
            false,
        );
    }

    #[test]
    fn dashboard_review_filters_are_clickable() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.show_dashboard_section(DashboardSection::AwaitingReview);

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 83, 2, &mut app),
            true,
        );
        assert_eq!(
            app.dashboard.review_scope(),
            crate::app::ReviewScope::Direct
        );
    }

    #[test]
    fn clicking_the_selected_dashboard_group_toggles_collapse() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Group { open: false, .. })
        ));

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Group { open: true, .. })
        ));
    }

    #[test]
    fn mock_error_keys_are_mock_only() {
        let mut mock_app = App::with_default_config(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::Char('5'), &mut mock_app), false);
        assert_continue_changed(key(KeyCode::F(1), &mut mock_app), true);
        assert!(mock_app.mock_debug_is_open());
        assert_continue_changed(key(KeyCode::Char('5'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert!(!mock_app.mock_debug_is_open());
        assert_continue_changed(key(KeyCode::F(1), &mut mock_app), true);
        assert_continue_changed(key(KeyCode::Char('0'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), None);

        let mut live_like_app = App::with_default_config(Box::new(EmptySource));
        assert_continue_changed(key(KeyCode::F(1), &mut live_like_app), false);
        assert!(!live_like_app.mock_debug_is_open());
        assert_eq!(live_like_app.mock_error_mode(), None);
    }

    #[test]
    fn mock_error_keys_replace_active_mock_load() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.dashboard.loading = true;

        assert_continue_changed(key(KeyCode::F(1), &mut app), true);
        assert_continue_changed(key(KeyCode::Char('5'), &mut app), true);

        assert_eq!(app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert!(app.dashboard.loading);
    }

    #[test]
    fn mock_debug_popup_closes_without_changing_mode() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::F(1), &mut app), true);
        assert_continue_changed(key(KeyCode::Esc, &mut app), true);

        assert!(!app.mock_debug_is_open());
        assert_eq!(app.mock_error_mode(), None);
    }

    #[test]
    fn dashboard_slash_opens_search_and_search_captures_shortcuts() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('/'), &mut app), true);
        assert!(app.search_is_open());

        assert_continue_changed(key(KeyCode::Char('r'), &mut app), true);
        assert_eq!(app.search_query(), Some("r"));
        assert!(!app.dashboard.loading);

        assert_continue_changed(key(KeyCode::Esc, &mut app), true);
        assert!(!app.search_is_open());
    }

    #[test]
    fn dashboard_theme_picker_previews_and_closes() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::Char('t'), &mut app), true);
        assert!(app.theme_picker_is_open());
        assert_eq!(app.active_theme_index(), 0);

        assert_continue_changed(key(KeyCode::Char('j'), &mut app), true);
        assert_eq!(app.selected_theme_index(), 1);
        assert_eq!(app.active_theme_index(), 1);

        assert_continue_changed(key(KeyCode::Esc, &mut app), true);
        assert!(!app.theme_picker_is_open());
        assert_eq!(app.active_theme_index(), 0);
    }

    #[test]
    fn dashboard_theme_picker_previews_clicked_theme() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.open_theme_picker();

        assert!(handle_theme_picker_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &mut app,
            Some(MouseTarget::Theme(2)),
        ));
        assert_eq!(app.selected_theme_index(), 2);
        assert_eq!(app.active_theme_index(), 2);

        assert!(!handle_theme_picker_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &mut app,
            Some(MouseTarget::DashboardRow(1)),
        ));
        assert_eq!(app.selected_theme_index(), 2);
    }

    #[test]
    fn search_keys_move_backspace_and_open_match() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.open_search();

        assert_continue_changed(key(KeyCode::Char('x'), &mut app), true);
        assert_continue_changed(key(KeyCode::Backspace, &mut app), true);
        assert_eq!(app.search_query(), Some(""));

        assert_continue_changed(key(KeyCode::Down, &mut app), true);
        assert_eq!(app.selected_search_index(), Some(1));
        assert_continue_changed(ctrl_key(KeyCode::Char('p'), &mut app), true);
        assert_eq!(app.selected_search_index(), Some(0));

        assert_continue_changed(key(KeyCode::Enter, &mut app), true);
        assert_eq!(app.view, AppView::Detail);
        assert!(!app.search_is_open());
    }

    #[test]
    fn search_result_click_opens_match() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();
        app.open_search();
        let expected = app.search_matches()[0].pr.clone();

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 16, 12, &mut app),
            true,
        );

        assert_eq!(app.view, AppView::Detail);
        assert!(!app.search_is_open());
        assert_eq!(
            app.detail.current.as_ref().map(|detail| &detail.pr),
            Some(&expected)
        );
    }

    #[test]
    fn detail_keys_switch_panes_scroll_and_return_to_dashboard() {
        let mut app = App::with_default_config(Box::new(EmptySource));
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
        handle_event(
            Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
            app,
            &MouseLayout::default(),
        )
        .unwrap()
    }

    fn ctrl_key(code: KeyCode, app: &mut App) -> InputOutcome {
        handle_event(
            Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL)),
            app,
            &MouseLayout::default(),
        )
        .unwrap()
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16, app: &mut App) -> InputOutcome {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        let mut mouse_layout = MouseLayout::default();
        terminal
            .draw(|frame| mouse_layout = render(frame, app))
            .unwrap();
        handle_event(
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            }),
            app,
            &mouse_layout,
        )
        .unwrap()
    }

    fn assert_continue_changed(outcome: InputOutcome, expected: bool) {
        match outcome {
            InputOutcome::Continue(changed) => assert_eq!(changed, expected),
            InputOutcome::Quit => panic!("expected continue outcome"),
        }
    }
}
