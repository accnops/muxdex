use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{execute, terminal};
use muxdex::app::{App, AppError};
use muxdex::tmux::{ProcessRunner, TmuxProbe};
use muxdex::ui;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let probe = TmuxProbe::new(ProcessRunner);
    let mut app = App::new();
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now() - tick_rate;

    loop {
        if last_tick.elapsed() >= tick_rate {
            app.apply_probe_result(probe.poll_sessions().map_err(AppError::from));
            last_tick = Instant::now();
        }

        terminal.draw(|frame| ui::draw(frame, &app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Tab
                | KeyCode::Right
                | KeyCode::Down
                | KeyCode::Char('j')
                | KeyCode::Char('l') => {
                    app.move_focus_next();
                }
                KeyCode::BackTab
                | KeyCode::Left
                | KeyCode::Up
                | KeyCode::Char('h')
                | KeyCode::Char('k') => {
                    app.move_focus_previous();
                }
                KeyCode::Esc => app.cancel_overlay(),
                KeyCode::Enter => {
                    if let Some(session) = app.overlay_session_name().map(str::to_owned) {
                        match probe.kill_session(&session) {
                            Ok(()) => {
                                app.hide_session(&session);
                                app.cancel_overlay();
                            }
                            Err(error) => {
                                app.set_error(AppError::non_fatal(error.message().to_string()));
                                app.cancel_overlay();
                            }
                        }
                    }
                }
                KeyCode::Char('x') => app.request_close_for_focused(),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                _ => {}
            }
        }
    }

    Ok(())
}
