use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::{Frame, layout::Alignment, style::Color};

use crate::app::{App, OverlayState};
use crate::layout::compute_grid;
use crate::store::{SessionRecord, SessionStatus};

pub fn render_empty_state_text(_: &App) -> String {
    "No tmux sessions detected. Tiles appear automatically when sessions start.".to_string()
}

fn footer_text() -> &'static str {
    "tab/shift-tab move  x close  q quit"
}

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let [content_area, footer_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    if let Some(error) = app.global_error() {
        if app.visible_sessions().is_empty() && error.is_fatal() {
            render_centered_message(
                frame,
                content_area,
                "tmux unavailable",
                &error.message().to_string(),
            );
            render_footer(frame, footer_area, app);
            return;
        }
    }

    let visible_sessions = app.visible_sessions();
    if visible_sessions.is_empty() {
        render_centered_message(frame, content_area, "muxdex", &render_empty_state_text(app));
    } else {
        for (record, area) in visible_sessions
            .into_iter()
            .zip(compute_grid(content_area, app.visible_sessions().len()))
        {
            render_tile(frame, area, record, app.focused_session_name().as_deref());
        }
    }

    render_footer(frame, footer_area, app);

    if let Some(OverlayState::ConfirmKill { session }) = app.overlay() {
        render_confirm_overlay(frame, session);
    }
}

fn render_tile(
    frame: &mut Frame<'_>,
    area: Rect,
    record: &SessionRecord,
    focused_name: Option<&str>,
) {
    let is_focused = focused_name == Some(record.name.as_str());
    let border_style = match record.status {
        SessionStatus::Live => Style::default().fg(record.accent),
        SessionStatus::Dead => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    };
    let block = Block::bordered()
        .border_set(tile_border_set(is_focused))
        .border_style(border_style)
        .title_top(tile_title(record, is_focused));
    let content = if let Some(reason) = &record.stale_reason {
        let mut text = record.snapshot.text().clone();
        text.lines.push(Line::from(vec![Span::styled(
            format!("stale: {reason}"),
            Style::default().fg(Color::Yellow),
        )]));
        text
    } else {
        record.snapshot.text().clone()
    };
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(match record.status {
            SessionStatus::Live => Style::default(),
            SessionStatus::Dead => Style::default().add_modifier(Modifier::DIM),
        });

    frame.render_widget(paragraph, area);
}

fn tile_border_set(is_focused: bool) -> border::Set<'static> {
    if is_focused {
        border::DOUBLE
    } else {
        border::ROUNDED
    }
}

fn tile_title(record: &SessionRecord, is_focused: bool) -> Line<'static> {
    let mut spans = vec![Span::styled(
        record.name.clone(),
        if is_focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        },
    )];

    if record.status == SessionStatus::Dead {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[killed]", Style::default().fg(Color::Gray)));
    }

    Line::from(spans)
}

fn render_centered_message(frame: &mut Frame<'_>, area: Rect, title: &str, message: &str) {
    let [vertical] = Layout::vertical([Constraint::Length(5)])
        .flex(Flex::Center)
        .areas(area);
    let [horizontal] = Layout::horizontal([Constraint::Percentage(70)])
        .flex(Flex::Center)
        .areas(vertical);
    let text = Paragraph::new(Text::from(vec![
        Line::from(title.bold()),
        Line::from(""),
        Line::from(message),
    ]))
    .alignment(Alignment::Center)
    .block(Block::bordered().border_set(border::ROUNDED));
    frame.render_widget(text, horizontal);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut parts = vec![Span::raw(footer_text())];

    if let Some(error) = app.global_error() {
        parts.extend([
            Span::raw("  "),
            Span::styled(error.message().to_string(), Style::default().fg(Color::Red)),
        ]);
    }

    frame.render_widget(Paragraph::new(Line::from(parts)), area);
}

fn render_confirm_overlay(frame: &mut Frame<'_>, session: &str) {
    let [vertical] = Layout::vertical([Constraint::Length(5)])
        .flex(Flex::Center)
        .areas(frame.area());
    let [horizontal] = Layout::horizontal([Constraint::Length(44)])
        .flex(Flex::Center)
        .areas(vertical);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from(format!("Kill tmux session '{session}'?").bold()),
        Line::from(""),
        Line::from("Enter confirms. Esc cancels."),
    ]))
    .alignment(Alignment::Center)
    .block(Block::bordered().title("Confirm kill"));
    frame.render_widget(Clear, horizontal);
    frame.render_widget(dialog, horizontal);
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use ratatui::{
        style::{Color, Modifier},
        symbols::border,
    };

    use crate::{
        app::App,
        store::{SessionRecord, SessionStatus},
        tmux::PaneSnapshot,
    };

    use super::{footer_text, render_empty_state_text, tile_border_set, tile_title};

    #[test]
    fn empty_state_explains_that_sessions_appear_automatically() {
        let app = App::new();
        let text = render_empty_state_text(&app);
        assert!(text.contains("No tmux sessions detected"));
        assert!(text.contains("appear automatically"));
    }

    #[test]
    fn focused_tile_uses_double_border_set() {
        assert_eq!(tile_border_set(true), border::DOUBLE);
        assert_eq!(tile_border_set(false), border::ROUNDED);
    }

    #[test]
    fn tile_title_does_not_include_close_glyph() {
        let title = tile_title(&sample_record(), true);
        let rendered = title
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("alpha"));
        assert!(!rendered.contains('X'));
    }

    #[test]
    fn live_tile_title_omits_status_label() {
        let title = tile_title(&sample_record(), true);
        let rendered = title
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("alpha"));
        assert!(!rendered.contains("[live]"));
    }

    #[test]
    fn dead_tile_title_uses_killed_label() {
        let title = tile_title(&dead_record(), false);
        let rendered = title
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("[killed]"));
        assert!(!rendered.contains("[dead]"));
    }

    #[test]
    fn unfocused_tile_title_is_not_bold() {
        let title = tile_title(&sample_record(), false);

        assert!(!title.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn focused_tile_title_stays_bold() {
        let title = tile_title(&sample_record(), true);

        assert!(title.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn footer_omits_confirm_and_cancel_hints() {
        let footer = footer_text();

        assert!(footer.contains("tab/shift-tab move"));
        assert!(footer.contains("x close"));
        assert!(footer.contains("q quit"));
        assert!(!footer.contains("enter confirm"));
        assert!(!footer.contains("esc cancel"));
    }

    fn sample_record() -> SessionRecord {
        SessionRecord {
            name: "alpha".to_string(),
            active_pane_id: "%1".to_string(),
            status: SessionStatus::Live,
            last_seen: SystemTime::UNIX_EPOCH,
            snapshot: PaneSnapshot::placeholder("hello", 10, 3),
            hidden: false,
            accent: Color::Cyan,
            stale_reason: None,
        }
    }

    fn dead_record() -> SessionRecord {
        SessionRecord {
            status: SessionStatus::Dead,
            ..sample_record()
        }
    }
}
