use super::dashboard::{dashboard_section_at_screen_position, row_index_at_screen_line};
use super::theme;
use crate::app::{App, AppView, DashboardSection, DetailPane};
use crate::github::MockErrorMode;
use anyhow::Result;
use crossterm::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::terminal;

pub(super) enum InputOutcome {
    Continue(bool),
    Quit,
}

pub(super) fn handle_event(event: Event, app: &mut App) -> Result<InputOutcome> {
    let key = match event {
        Event::Key(key) => key,
        Event::Mouse(mouse) => return Ok(InputOutcome::Continue(handle_mouse(mouse, app)?)),
        Event::Resize(_, _) => return Ok(InputOutcome::Continue(true)),
        _ => return Ok(InputOutcome::Continue(false)),
    };

    if key.kind != KeyEventKind::Press {
        return Ok(InputOutcome::Continue(false));
    }

    if app.theme_picker_is_open() {
        let changed = match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                app.close_theme_picker();
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
            KeyCode::Char('1') if app.config().dashboard.separate_views => {
                app.show_dashboard_section(DashboardSection::MyPrs)
            }
            KeyCode::Char('2') if app.config().dashboard.separate_views => {
                app.show_dashboard_section(DashboardSection::AwaitingReview)
            }
            KeyCode::Tab if app.config().dashboard.separate_views => app.cycle_dashboard_section(),
            KeyCode::Char('/') => {
                app.open_search();
                true
            }
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
            KeyCode::Char(key)
                if app.is_mock()
                    && mock_error_mode_for_key(key, app.config().dashboard.separate_views)
                        .is_some() =>
            {
                app.set_mock_error_mode(
                    mock_error_mode_for_key(key, app.config().dashboard.separate_views).flatten(),
                );
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

fn handle_mouse(mouse: MouseEvent, app: &mut App) -> Result<bool> {
    if app.theme_picker_is_open() {
        return Ok(match mouse.kind {
            MouseEventKind::ScrollDown => {
                app.next_theme(theme::theme_count());
                true
            }
            MouseEventKind::ScrollUp => {
                app.previous_theme(theme::theme_count());
                true
            }
            _ => false,
        });
    }

    if app.search_is_open() {
        return Ok(match mouse.kind {
            MouseEventKind::ScrollDown => {
                app.next_search_match();
                true
            }
            MouseEventKind::ScrollUp => {
                app.previous_search_match();
                true
            }
            _ => false,
        });
    }

    let changed = match app.view {
        AppView::Dashboard => handle_dashboard_mouse(mouse, app),
        AppView::Detail => handle_detail_mouse(mouse, app)?,
    };

    Ok(changed)
}

fn handle_dashboard_mouse(mouse: MouseEvent, app: &mut App) -> bool {
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
            if let Some(section) =
                dashboard_section_at_screen_position(app, mouse.column, mouse.row)
            {
                return app.show_dashboard_section(section);
            }

            let rows = app.rows();
            let Some(index) = row_index_at_screen_line(
                &rows,
                mouse.row,
                app.dashboard.scroll,
                app.config().dashboard.separate_views,
            ) else {
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

fn handle_detail_mouse(mouse: MouseEvent, app: &mut App) -> Result<bool> {
    let Some(pane) = detail_pane_at_row(mouse.row)? else {
        return Ok(false);
    };

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            app.focus_detail_pane(pane);
            app.scroll_active_down();
            Ok(true)
        }
        MouseEventKind::ScrollUp => {
            app.focus_detail_pane(pane);
            app.scroll_active_up();
            Ok(true)
        }
        MouseEventKind::Down(MouseButton::Left) => {
            app.focus_detail_pane(pane);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn detail_pane_at_row(row: u16) -> Result<Option<DetailPane>> {
    let (_, height) = terminal::size()?;
    if height <= 2 || row >= height.saturating_sub(2) {
        return Ok(None);
    }

    let description_height = height.saturating_sub(2) / 2;
    if row < description_height {
        Ok(Some(DetailPane::Description))
    } else {
        Ok(Some(DetailPane::Discussion))
    }
}

fn mock_error_mode_for_key(
    key: char,
    separate_dashboard_views: bool,
) -> Option<Option<MockErrorMode>> {
    if separate_dashboard_views {
        match key {
            '0' => Some(None),
            '5' => Some(Some(MockErrorMode::GitHubDown)),
            '6' => Some(Some(MockErrorMode::Timeout)),
            '7' => Some(Some(MockErrorMode::Generic)),
            '8' => Some(Some(MockErrorMode::Auth)),
            _ => None,
        }
    } else {
        match key {
            '0' => Some(None),
            '1' => Some(Some(MockErrorMode::GitHubDown)),
            '2' => Some(Some(MockErrorMode::Timeout)),
            '3' => Some(Some(MockErrorMode::Generic)),
            '4' => Some(Some(MockErrorMode::Auth)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DetailPane, DetailStatus};
    use crate::config::Config;
    use crate::github::{GhStatus, MockErrorMode, MockGhClient, PullRequestSource};
    use crate::model::{PullRequest, PullRequestDetail};
    use anyhow::Result;
    use crossterm::event::{KeyEvent, KeyEventState, KeyModifiers, MouseEvent};

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

    fn separate_views_config() -> Config {
        let mut config = Config::default();
        config.dashboard.separate_views = true;
        config
    }

    #[test]
    fn resize_requests_redraw_and_key_release_is_ignored() {
        let mut app = App::with_default_config(Box::new(EmptySource));

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
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('j'), &mut app), true);
        assert_eq!(app.dashboard.selected, 1);

        assert_continue_changed(key(KeyCode::Char('o'), &mut app), true);
        assert!(matches!(
            app.rows().get(1),
            Some(crate::app::Row::Group { open: false, .. })
        ));

        assert_continue_changed(key(KeyCode::Char('k'), &mut app), true);
        assert_eq!(app.dashboard.selected, 0);
        assert!(matches!(
            key(KeyCode::Char('q'), &mut app),
            InputOutcome::Quit
        ));
    }

    #[test]
    fn separate_dashboard_view_keys_switch_sections() {
        let mut app = App::new(Box::new(MockGhClient::new()), separate_views_config());
        app.refresh();

        assert_continue_changed(key(KeyCode::Char('2'), &mut app), true);
        assert_eq!(
            app.dashboard.active_section(),
            DashboardSection::AwaitingReview
        );
        assert_eq!(app.mock_error_mode(), None);

        assert_continue_changed(key(KeyCode::Char('5'), &mut app), true);
        assert_eq!(app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert_continue_changed(key(KeyCode::Char('0'), &mut app), true);
        assert_eq!(app.mock_error_mode(), None);

        assert_continue_changed(key(KeyCode::Char('1'), &mut app), true);
        assert_eq!(app.dashboard.active_section(), DashboardSection::MyPrs);

        assert_continue_changed(key(KeyCode::Tab, &mut app), true);
        assert_eq!(
            app.dashboard.active_section(),
            DashboardSection::AwaitingReview
        );
    }

    #[test]
    fn search_captures_dashboard_view_keys() {
        let mut app = App::new(Box::new(MockGhClient::new()), separate_views_config());
        app.refresh();
        app.open_search();

        assert_continue_changed(key(KeyCode::Char('2'), &mut app), true);

        assert_eq!(app.search_query(), Some("2"));
        assert_eq!(app.dashboard.active_section(), DashboardSection::MyPrs);
    }

    #[test]
    fn dashboard_mouse_wheel_scrolls_and_click_selects_or_opens_rows() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(mouse(MouseEventKind::ScrollDown, 0, 0, &mut app), true);
        assert_eq!(app.dashboard.selected, 0);
        assert_eq!(app.dashboard.scroll, 1);

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Pr(_))
        ));

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert_eq!(app.view, AppView::Detail);
    }

    #[test]
    fn dashboard_tab_clicks_switch_separate_views() {
        let mut app = App::new(Box::new(MockGhClient::new()), separate_views_config());
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
    fn dashboard_mouse_second_click_on_group_toggles_collapse() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.refresh();

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Group { open: true, .. })
        ));

        assert_continue_changed(
            mouse(MouseEventKind::Down(MouseButton::Left), 4, 4, &mut app),
            true,
        );
        assert!(matches!(
            app.rows().get(app.dashboard.selected),
            Some(crate::app::Row::Group { open: false, .. })
        ));
    }

    #[test]
    fn mock_error_keys_are_mock_only() {
        let mut mock_app = App::with_default_config(Box::new(MockGhClient::new()));

        assert_continue_changed(key(KeyCode::Char('1'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert_continue_changed(key(KeyCode::Char('0'), &mut mock_app), true);
        assert_eq!(mock_app.mock_error_mode(), None);

        let mut live_like_app = App::with_default_config(Box::new(EmptySource));
        assert_continue_changed(key(KeyCode::Char('1'), &mut live_like_app), false);
        assert_eq!(live_like_app.mock_error_mode(), None);
    }

    #[test]
    fn mock_error_keys_replace_active_mock_load() {
        let mut app = App::with_default_config(Box::new(MockGhClient::new()));
        app.dashboard.loading = true;

        assert_continue_changed(key(KeyCode::Char('1'), &mut app), true);

        assert_eq!(app.mock_error_mode(), Some(MockErrorMode::GitHubDown));
        assert!(app.dashboard.loading);
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
        assert_eq!(app.active_theme_index(), 1);
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
        handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), app).unwrap()
    }

    fn ctrl_key(code: KeyCode, app: &mut App) -> InputOutcome {
        handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL)), app).unwrap()
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16, app: &mut App) -> InputOutcome {
        handle_event(
            Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            }),
            app,
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
