use std::time::SystemTime;

use crate::store::{SessionRecord, SessionStore};
use crate::tmux::{ObservedSession, ProbeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayState {
    ConfirmKill { session: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    message: String,
    fatal: bool,
}

impl AppError {
    pub fn tmux_unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: true,
        }
    }

    pub fn non_fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fatal: false,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn is_fatal(&self) -> bool {
        self.fatal
    }
}

impl From<ProbeError> for AppError {
    fn from(value: ProbeError) -> Self {
        Self::tmux_unavailable(value.message().to_string())
    }
}

pub struct App {
    store: SessionStore,
    overlay: Option<OverlayState>,
    focus_index: usize,
    global_error: Option<AppError>,
}

impl App {
    pub fn new() -> Self {
        Self {
            store: SessionStore::default(),
            overlay: None,
            focus_index: 0,
            global_error: None,
        }
    }

    pub fn visible_sessions(&self) -> Vec<&SessionRecord> {
        self.store.visible_records()
    }

    pub fn overlay(&self) -> Option<&OverlayState> {
        self.overlay.as_ref()
    }

    pub fn global_error(&self) -> Option<&AppError> {
        self.global_error.as_ref()
    }

    pub fn request_close_for_focused(&mut self) {
        let Some(session) = self.focused_session_name() else {
            return;
        };

        if self.store.is_live(&session) {
            self.overlay = Some(OverlayState::ConfirmKill { session });
        } else {
            self.store.hide(&session);
            self.normalize_focus();
        }
    }

    pub fn cancel_overlay(&mut self) {
        self.overlay = None;
    }

    pub fn apply_probe_result(&mut self, result: Result<Vec<ObservedSession>, AppError>) {
        match result {
            Ok(observed) => {
                self.global_error = None;
                self.store.reconcile(observed, SystemTime::now());
                self.normalize_focus();
            }
            Err(error) => {
                self.global_error = Some(error);
            }
        }
    }

    pub fn move_focus_next(&mut self) {
        let visible = self.visible_sessions();
        if visible.is_empty() {
            self.focus_index = 0;
            return;
        }

        self.focus_index = (self.focus_index + 1) % visible.len();
    }

    pub fn move_focus_previous(&mut self) {
        let visible = self.visible_sessions();
        if visible.is_empty() {
            self.focus_index = 0;
            return;
        }

        self.focus_index = if self.focus_index == 0 {
            visible.len() - 1
        } else {
            self.focus_index - 1
        };
    }

    pub fn focused_session_name(&self) -> Option<String> {
        self.visible_sessions()
            .get(self.focus_index)
            .map(|record| record.name.clone())
    }

    pub fn overlay_session_name(&self) -> Option<&str> {
        match self.overlay.as_ref() {
            Some(OverlayState::ConfirmKill { session }) => Some(session.as_str()),
            None => None,
        }
    }

    pub fn hide_session(&mut self, name: &str) {
        self.store.hide(name);
        self.normalize_focus();
    }

    pub fn set_error(&mut self, error: AppError) {
        self.global_error = Some(error);
    }

    fn normalize_focus(&mut self) {
        let visible_len = self.visible_sessions().len();
        if visible_len == 0 {
            self.focus_index = 0;
        } else if self.focus_index >= visible_len {
            self.focus_index = visible_len - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use crate::tmux::{ObservedSession, PaneSize, PaneSnapshot};

    use super::{App, AppError, OverlayState};

    #[test]
    fn new_app_starts_with_empty_visible_sessions() {
        let app = App::new();

        assert!(app.visible_sessions().is_empty());
        assert!(app.overlay().is_none());
    }

    #[test]
    fn confirm_close_requires_explicit_acceptance_for_live_session() {
        let mut app = app_with_live_session("alpha");
        app.request_close_for_focused();

        assert!(
            matches!(app.overlay(), Some(OverlayState::ConfirmKill { session }) if session == "alpha")
        );

        app.cancel_overlay();
        assert!(app.overlay().is_none());
    }

    #[test]
    fn app_surfaces_probe_errors_without_clearing_existing_tiles() {
        let mut app = app_with_live_session("alpha");

        app.apply_probe_result(Err(AppError::tmux_unavailable("tmux missing")));

        assert!(app.global_error().is_some());
        assert_eq!(app.visible_sessions().len(), 1);
    }

    fn app_with_live_session(name: &str) -> App {
        let mut app = App::new();
        app.apply_probe_result(Ok(vec![ObservedSession {
            name: name.to_string(),
            active_pane_id: "%1".to_string(),
            pane_size: PaneSize {
                width: 80,
                height: 24,
            },
            capture: Ok(PaneSnapshot::from_tmux_capture("hello", 80, 24)),
        }]));
        app
    }

    fn _timestamp(seconds: u64) -> std::time::SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }
}
