//! Play Nine scorekeeper TUI — not a card-play engine.
//!
//! Persistence: `data/play_nine.json` (created at runtime under the CWD).

mod app;
mod persist;
mod sayings;
mod scenes;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, JumpPhase, Screen};

fn main() {
    if let Err(e) = run() {
        eprintln!("play-nine-tui error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut app = App::boot().map_err(|e| {
        format!(
            "{e}\n\nHint: delete or repair {} and restart.",
            persist::data_path().display()
        )
    })?;

    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture).map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let result = event_loop(&mut terminal, &mut app);

    // Always restore terminal.
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|f| ui::draw(f, app))
            .map_err(|e| e.to_string())?;

        if app.screen == Screen::Celebration {
            app.tick_celebration();
        }

        if !event::poll(Duration::from_millis(120)).map_err(|e| e.to_string())? {
            continue;
        }

        let Event::Key(key) = event::read().map_err(|e| e.to_string())? else {
            continue;
        };
        // Ignore key-release / repeats on some terminals.
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if handle_key(app, key.code)? {
            let _ = app.persist();
            break;
        }
    }
    Ok(())
}

/// Returns true when the app should exit.
fn handle_key(app: &mut App, code: KeyCode) -> Result<bool, String> {
    match app.screen {
        Screen::MainMenu => match code {
            KeyCode::Up | KeyCode::Char('k') => app.menu_up(),
            KeyCode::Down | KeyCode::Char('j') => app.menu_down(),
            KeyCode::Enter => app.menu_select(),
            KeyCode::Char(c) if matches!(c, 'n' | 'N' | 'r' | 'R' | 'h' | 'H' | 'q' | 'Q') => {
                app.menu_activate_by_letter(c);
            }
            _ => {}
        },
        Screen::SetupHoles => match code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char(' ') => {
                app.toggle_holes()
            }
            KeyCode::Enter => app.confirm_holes(),
            KeyCode::Esc => app.back_to_menu(),
            _ => {}
        },
        Screen::SetupPlayers => match code {
            KeyCode::Esc => app.back_to_menu(),
            KeyCode::Enter => {
                if app.name_buf.trim().is_empty() {
                    app.start_game_from_names();
                } else {
                    app.add_player_name();
                }
            }
            KeyCode::Backspace => {
                if app.name_buf.is_empty() {
                    app.remove_last_pending_name();
                } else {
                    app.name_buf.pop();
                }
            }
            KeyCode::Char(c) if !c.is_control() => {
                if app.name_buf.len() < 24 {
                    app.name_buf.push(c);
                }
            }
            _ => {}
        },
        Screen::Scoring => handle_scoring_key(app, code),
        Screen::Celebration => {
            // Any key leaves the 19th hole.
            app.leave_celebration();
        }
        Screen::History => match code {
            KeyCode::Esc => app.back_to_menu(),
            KeyCode::Up | KeyCode::Char('k') => {
                app.history_scroll = app.history_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.history_scroll + 1 < app.history.len() {
                    app.history_scroll += 1;
                }
            }
            _ => {}
        },
        Screen::QuitConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.screen = Screen::MainMenu;
            }
            _ => {}
        },
    }
    Ok(false)
}

fn handle_scoring_key(app: &mut App, code: KeyCode) {
    match app.jump_phase {
        JumpPhase::Hole | JumpPhase::Player => match code {
            KeyCode::Esc => app.cancel_jump(),
            KeyCode::Enter => {
                if app.jump_phase == JumpPhase::Hole {
                    app.submit_jump_hole();
                } else {
                    app.submit_jump_player();
                }
            }
            KeyCode::Backspace => {
                app.jump_buf.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if app.jump_buf.len() < 3 {
                    app.jump_buf.push(c);
                }
            }
            _ => {}
        },
        JumpPhase::Idle => match code {
            KeyCode::Esc => app.back_to_menu(),
            KeyCode::Enter => app.submit_score(),
            KeyCode::Backspace => {
                app.score_buf.pop();
            }
            KeyCode::Char('e') | KeyCode::Char('E') => app.begin_jump(),
            KeyCode::Char(c) if c == '-' || c.is_ascii_digit() => {
                if app.score_buf.len() < 4 {
                    app.score_buf.push(c);
                }
            }
            _ => {}
        },
    }
}
