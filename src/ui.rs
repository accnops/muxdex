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
    let status = match record.status {
        SessionStatus::Live => "live",
        SessionStatus::Dead => "dead",
    };
    let is_focused = focused_name == Some(record.name.as_str());
    let border_style = match record.status {
        SessionStatus::Live => Style::default().fg(record.accent),
        SessionStatus::Dead => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    };
    let title = Line::from(vec![
        Span::styled(
            record.name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("[{status}]"), Style::default().fg(Color::Gray)),
        Span::raw(" "),
        Span::styled("X", Style::default().fg(Color::Red)),
    ]);
    let block = Block::bordered()
        .border_set(if is_focused {
            border::THICK
        } else {
            border::ROUNDED
        })
        .border_style(border_style)
        .title_top(title);
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
    let mut parts = vec![
        Span::raw("tab/shift-tab move"),
        Span::raw("  "),
        Span::raw("x close"),
        Span::raw("  "),
        Span::raw("enter confirm"),
        Span::raw("  "),
        Span::raw("esc cancel"),
        Span::raw("  "),
        Span::raw("q quit"),
    ];

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
    use crate::app::App;

    use super::render_empty_state_text;

    #[test]
    fn empty_state_explains_that_sessions_appear_automatically() {
        let app = App::new();
        let text = render_empty_state_text(&app);

        assert!(text.contains("No tmux sessions detected"));
        assert!(text.contains("appear automatically"));
    }
}
