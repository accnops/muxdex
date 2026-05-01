use std::ffi::OsStr;
use std::io;
use std::process::Command;

use ansi_to_tui::IntoText as _;
use anyhow::{Context, anyhow};
use ratatui::text::Text;

const LIST_PANES_FORMAT: &str =
    "#{session_name}|#{pane_id}|#{pane_width}|#{pane_height}|#{window_active}|#{pane_active}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSize {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxSessionRow {
    pub name: String,
    pub active_pane_id: String,
    pub pane_size: PaneSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSnapshot {
    width: u16,
    height: u16,
    text: Text<'static>,
    plain_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSession {
    pub name: String,
    pub active_pane_id: String,
    pub pane_size: PaneSize,
    pub capture: Result<PaneSnapshot, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeError {
    message: String,
}

impl ProbeError {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn is_no_server_running(&self) -> bool {
        self.message.contains("no server running")
    }
}

pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, ProbeError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessRunner;

pub struct TmuxProbe<R> {
    runner: R,
}

impl<R> TmuxProbe<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl TmuxProbe<ProcessRunner> {
    pub fn system() -> Self {
        Self::new(ProcessRunner)
    }
}

impl<R: CommandRunner> TmuxProbe<R> {
    pub fn poll_sessions(&self) -> Result<Vec<ObservedSession>, ProbeError> {
        let rows = match self
            .runner
            .run(&["list-panes", "-a", "-F", LIST_PANES_FORMAT])
        {
            Ok(rows) => rows,
            Err(error) if error.is_no_server_running() => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };

        if rows.contains("no server running") {
            return Ok(Vec::new());
        }

        let sessions = parse_session_rows(&rows).map_err(|error| {
            ProbeError::from_message(format!("failed to parse tmux sessions: {error:#}"))
        })?;

        Ok(sessions
            .into_iter()
            .map(|session| {
                let capture = self
                    .runner
                    .run(&["capture-pane", "-e", "-p", "-t", &session.active_pane_id])
                    .map(|raw| {
                        PaneSnapshot::from_tmux_capture(
                            &raw,
                            session.pane_size.width,
                            session.pane_size.height,
                        )
                    })
                    .map_err(|error| error.message().to_string());

                ObservedSession {
                    name: session.name,
                    active_pane_id: session.active_pane_id,
                    pane_size: session.pane_size,
                    capture,
                }
            })
            .collect())
    }

    pub fn kill_session(&self, session_name: &str) -> Result<(), ProbeError> {
        self.runner
            .run(&["kill-session", "-t", session_name])
            .map(|_| ())
    }
}

impl ProcessRunner {
    fn command_failure(stderr: &str) -> ProbeError {
        if stderr.contains("no server running") {
            ProbeError::from_message(stderr.trim())
        } else {
            ProbeError::from_message(format!("tmux command failed: {}", stderr.trim()))
        }
    }
}

impl CommandRunner for ProcessRunner {
    fn run(&self, args: &[&str]) -> Result<String, ProbeError> {
        let output = Command::new("tmux")
            .args(args.iter().map(OsStr::new))
            .output()
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => {
                    ProbeError::from_message("tmux is not installed or not on PATH")
                }
                _ => ProbeError::from_message(format!("failed to run tmux: {error}")),
            })?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }

        Err(Self::command_failure(&String::from_utf8_lossy(
            &output.stderr,
        )))
    }
}

impl ProbeError {
    fn from_message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl PaneSnapshot {
    pub fn from_tmux_capture(raw: &str, width: u16, height: u16) -> Self {
        let text = raw
            .into_text()
            .unwrap_or_else(|_| Text::raw(strip_ansi_escapes(raw)));
        let plain_lines = normalize_newlines(&strip_ansi_escapes(raw))
            .into_iter()
            .collect();

        Self {
            width,
            height,
            text,
            plain_lines,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.plain_lines
    }

    pub fn plain_text(&self) -> String {
        self.plain_lines.join("\n")
    }

    pub fn text(&self) -> &Text<'static> {
        &self.text
    }

    pub fn placeholder(message: &str, width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            text: Text::raw(message.to_string()),
            plain_lines: vec![message.to_string()],
        }
    }
}

pub fn parse_session_rows(rows: &str) -> anyhow::Result<Vec<TmuxSessionRow>> {
    rows.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_session_row)
        .collect()
}

fn parse_session_row(line: &str) -> Option<anyhow::Result<TmuxSessionRow>> {
    let parts: Vec<&str> = line.split('|').collect();
    match parts.as_slice() {
        [name, active_pane_id, width, height] => {
            Some(build_session_row(name, active_pane_id, width, height))
        }
        [
            name,
            active_pane_id,
            width,
            height,
            window_active,
            pane_active,
        ] => {
            if *window_active == "1" && *pane_active == "1" {
                Some(build_session_row(name, active_pane_id, width, height))
            } else {
                None
            }
        }
        _ => Some(Err(anyhow!("invalid tmux row: {line}"))),
    }
}

fn build_session_row(
    name: &str,
    active_pane_id: &str,
    width: &str,
    height: &str,
) -> anyhow::Result<TmuxSessionRow> {
    Ok(TmuxSessionRow {
        name: name.split(':').next().unwrap_or(name).to_string(),
        active_pane_id: active_pane_id.to_string(),
        pane_size: PaneSize {
            width: width.parse::<u16>().context("invalid pane width")?,
            height: height.parse::<u16>().context("invalid pane height")?,
        },
    })
}

fn strip_ansi_escapes(raw: &str) -> String {
    let mut output = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            output.push(ch);
            continue;
        }

        if chars.peek() == Some(&'[') {
            chars.next();
            for escape in chars.by_ref() {
                if ('@'..='~').contains(&escape) {
                    break;
                }
            }
        }
    }

    output
}

fn normalize_newlines(raw: &str) -> Vec<String> {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_end_matches('\n')
        .split('\n')
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        CommandRunner, LIST_PANES_FORMAT, PaneSnapshot, ProbeError, TmuxProbe, parse_session_rows,
    };

    #[test]
    fn parse_sessions_extracts_names_and_active_panes() {
        let output = "alpha|%1|120|30\nbeta|%8|200|60\n";
        let sessions = parse_session_rows(output).unwrap();

        assert_eq!(sessions[0].name, "alpha");
        assert_eq!(sessions[0].active_pane_id, "%1");
        assert_eq!(sessions[1].pane_size.width, 200);
    }

    #[test]
    fn parse_snapshot_preserves_cells_and_lines() {
        let raw = "one\ntwo\n";
        let snapshot = PaneSnapshot::from_tmux_capture(raw, 8, 2);

        assert_eq!(snapshot.lines()[0], "one");
        assert_eq!(snapshot.lines()[1], "two");
    }

    #[test]
    fn poll_sessions_preserves_capture_failures_per_session() {
        let probe = TmuxProbe::new(FakeRunner::new([
            (
                format!("list-panes\0-a\0-F\0{LIST_PANES_FORMAT}"),
                Ok("alpha|%1|80|24|1|1\nbeta|%2|80|24|1|1\n".to_string()),
            ),
            (
                "capture-pane\0-e\0-p\0-t\0%1".to_string(),
                Ok("alpha output".to_string()),
            ),
            (
                "capture-pane\0-e\0-p\0-t\0%2".to_string(),
                Err("capture failed".to_string()),
            ),
        ]));

        let observed = probe.poll_sessions().unwrap();

        assert_eq!(observed.len(), 2);
        assert_eq!(
            observed[0].capture.as_ref().unwrap().plain_text(),
            "alpha output"
        );
        assert!(
            matches!(observed[1].capture, Err(ref message) if message.contains("capture failed"))
        );
    }

    #[test]
    fn poll_sessions_returns_empty_when_no_tmux_server_is_running() {
        let probe = TmuxProbe::new(FakeRunner::new([(
            format!("list-panes\0-a\0-F\0{LIST_PANES_FORMAT}"),
            Ok("no server running on /tmp/tmux-501/default\n".to_string()),
        )]));

        let observed = probe.poll_sessions().unwrap();

        assert!(observed.is_empty());
    }

    #[test]
    fn poll_sessions_returns_empty_when_no_tmux_server_is_reported_as_error() {
        let probe = TmuxProbe::new(FakeRunner::new([(
            format!("list-panes\0-a\0-F\0{LIST_PANES_FORMAT}"),
            Err("no server running on /private/tmp/tmux-501/default".to_string()),
        )]));

        let observed = probe.poll_sessions().unwrap();

        assert!(observed.is_empty());
    }

    struct FakeRunner {
        responses: BTreeMap<String, Result<String, String>>,
    }

    impl FakeRunner {
        fn new<const N: usize>(entries: [(String, Result<String, String>); N]) -> Self {
            Self {
                responses: entries.into_iter().collect(),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, args: &[&str]) -> Result<String, ProbeError> {
            self.responses
                .get(&args.join("\0"))
                .cloned()
                .unwrap_or_else(|| Err("missing fake response".to_string()))
                .map_err(ProbeError::from_message)
        }
    }
}
