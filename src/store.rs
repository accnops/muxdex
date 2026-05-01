use std::collections::{BTreeMap, HashSet};
use std::time::SystemTime;

use ratatui::style::Color;

use crate::tmux::{ObservedSession, PaneSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Live,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub name: String,
    pub active_pane_id: String,
    pub status: SessionStatus,
    pub last_seen: SystemTime,
    pub snapshot: PaneSnapshot,
    pub hidden: bool,
    pub accent: Color,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    records: BTreeMap<String, SessionRecord>,
}

impl SessionStore {
    pub fn reconcile(&mut self, observed: Vec<ObservedSession>, now: SystemTime) {
        let live_names: HashSet<String> = observed
            .iter()
            .map(|session| session.name.clone())
            .collect();

        for session in observed {
            self.upsert_live(session, now);
        }

        for record in self.records.values_mut() {
            if !live_names.contains(&record.name) {
                record.status = SessionStatus::Dead;
            }
        }
    }

    pub fn upsert_live(&mut self, session: ObservedSession, now: SystemTime) {
        let accent = accent_for_name(&session.name);
        let name = session.name.clone();
        let capture = session.capture;

        let record = self
            .records
            .entry(name.clone())
            .or_insert_with(|| SessionRecord {
                name: name.clone(),
                active_pane_id: session.active_pane_id.clone(),
                status: SessionStatus::Live,
                last_seen: now,
                snapshot: fallback_snapshot(),
                hidden: false,
                accent,
                stale_reason: None,
            });

        record.name = name;
        record.active_pane_id = session.active_pane_id;
        record.status = SessionStatus::Live;
        record.last_seen = now;
        record.hidden = false;
        record.accent = accent;

        match capture {
            Ok(snapshot) => {
                record.snapshot = snapshot;
                record.stale_reason = None;
            }
            Err(error) => {
                record.stale_reason = Some(error);
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&SessionRecord> {
        self.records.get(name)
    }

    pub fn hide(&mut self, name: &str) {
        if let Some(record) = self.records.get_mut(name) {
            record.hidden = true;
        }
    }

    pub fn is_live(&self, name: &str) -> bool {
        self.records.get(name).is_some_and(SessionRecord::is_live)
    }

    pub fn visible_records(&self) -> Vec<&SessionRecord> {
        self.records
            .values()
            .filter(|record| !record.hidden)
            .collect()
    }
}

impl SessionRecord {
    pub fn is_live(&self) -> bool {
        self.status == SessionStatus::Live
    }

    pub fn is_dead(&self) -> bool {
        self.status == SessionStatus::Dead
    }
}

fn accent_for_name(name: &str) -> Color {
    const PALETTE: [Color; 6] = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::LightRed,
    ];

    let index = name.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    });

    PALETTE[index % PALETTE.len()]
}

fn fallback_snapshot() -> PaneSnapshot {
    PaneSnapshot::placeholder("Snapshot unavailable", 1, 1)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use crate::tmux::{ObservedSession, PaneSize, PaneSnapshot};

    use super::SessionStore;

    #[test]
    fn reconcile_marks_missing_session_dead_and_keeps_snapshot() {
        let mut store = SessionStore::default();
        store.upsert_live(sample_record("alpha", "still here"), timestamp(10));

        store.reconcile(Vec::new(), timestamp(20));

        let record = store.get("alpha").unwrap();
        assert!(record.is_dead());
        assert_eq!(record.snapshot.plain_text(), "still here");
    }

    #[test]
    fn reconcile_reactivates_dead_record_when_name_returns() {
        let mut store = SessionStore::default();
        store.upsert_live(sample_record("alpha", "before death"), timestamp(10));
        store.reconcile(Vec::new(), timestamp(20));

        store.reconcile(vec![sample_record("alpha", "after return")], timestamp(30));

        let record = store.get("alpha").unwrap();
        assert!(record.is_live());
        assert_eq!(record.snapshot.plain_text(), "after return");
    }

    fn sample_record(name: &str, text: &str) -> ObservedSession {
        ObservedSession {
            name: name.to_string(),
            active_pane_id: "%1".to_string(),
            pane_size: PaneSize {
                width: 80,
                height: 24,
            },
            capture: Ok(PaneSnapshot::from_tmux_capture(text, 80, 24)),
        }
    }

    fn timestamp(seconds: u64) -> std::time::SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }
}
