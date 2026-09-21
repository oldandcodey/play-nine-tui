//! Ratatui rendering — golf greens, fairway accents, ANSI-safe for SSH.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, JumpPhase, Screen};
use crate::scenes;

const GREEN: Color = Color::Green;
const FAIRWAY: Color = Color::Rgb(34, 139, 34);
const DEEP_GREEN: Color = Color::Rgb(0, 100, 0);
const SAND: Color = Color::Rgb(218, 165, 32);
const SKY: Color = Color::Cyan;
const FLAG: Color = Color::Red;
const MUTED: Color = Color::DarkGray;
const FOAM: Color = Color::Rgb(144, 238, 144);

/// Fixed inner width so left/right box borders always line up.
const BANNER_INNER: usize = 40;

fn title_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(FAIRWAY))
        .title(Span::styled(
            format!(" * {title} "),
            Style::default()
                .fg(GREEN)
                .add_modifier(Modifier::BOLD),
        ))
}

fn footer_hints(hints: &str) -> Paragraph<'_> {
    Paragraph::new(hints)
        .style(Style::default().fg(MUTED))
        .alignment(Alignment::Center)
}

fn pad_inner(s: &str) -> String {
    let mut out: String = s.chars().take(BANNER_INNER).collect();
    while out.chars().count() < BANNER_INNER {
        out.push(' ');
    }
    out
}

fn banner_border(top: bool) -> String {
    let fill = "═".repeat(BANNER_INNER);
    if top {
        format!("╔{fill}╗")
    } else {
        format!("╚{fill}╝")
    }
}

fn banner_row(inner: String, styles: Vec<(usize, usize, Style)>) -> Line<'static> {
    let mut spans = vec![Span::styled("║", Style::default().fg(FAIRWAY))];
    let mut cursor = 0usize;
    let chars: Vec<char> = inner.chars().collect();
    let mut styled: Vec<(usize, usize, Style)> = styles;
    styled.sort_by_key(|s| s.0);
    for (start, end, st) in styled {
        if cursor < start {
            let chunk: String = chars[cursor..start].iter().collect();
            spans.push(Span::styled(chunk, Style::default().fg(MUTED)));
        }
        let chunk: String = chars[start..end.min(chars.len())].iter().collect();
        spans.push(Span::styled(chunk, st));
        cursor = end.min(chars.len());
    }
    if cursor < chars.len() {
        let chunk: String = chars[cursor..].iter().collect();
        spans.push(Span::raw(chunk));
    }
    spans.push(Span::styled("║", Style::default().fg(FAIRWAY)));
    Line::from(spans)
}

/// Thin fairway stripe under the box banner (fixed-width ASCII).
fn fairway_accent() -> Line<'static> {
    // 42 cols to match ╔ + 40 + ╗ visual weight when centered.
    let bar = format!("~~{}~~", "=".repeat(36));
    Line::from(Span::styled(
        bar,
        Style::default().fg(FOAM).add_modifier(Modifier::DIM),
    ))
}

fn banner(frame: &mut Frame, area: Rect, app: &App) {
    let on_scoring = app.screen == Screen::Scoring && app.game.is_some();

    let title = pad_inner("  PLAY NINE  ·  SCOREKEEPER");
    let title_line = banner_row(
        title,
        vec![(
            2,
            27,
            Style::default()
                .fg(SKY)
                .add_modifier(Modifier::BOLD),
        )],
    );

    let sub = pad_inner("  lowest total wins · not a card eng.");
    let sub_line = banner_row(
        sub,
        vec![
            (2, 19, Style::default().fg(SAND)),
            (19, BANNER_INNER, Style::default().fg(MUTED)),
        ],
    );

    let mut lines = vec![
        Line::from(Span::styled(
            banner_border(true),
            Style::default().fg(FAIRWAY),
        )),
        title_line,
        sub_line,
        Line::from(Span::styled(
            banner_border(false),
            Style::default().fg(FAIRWAY),
        )),
        fairway_accent(),
    ];

    // Golf vignette strip while scoring (rotates on hole change).
    if on_scoring {
        let scene = scenes::scene_lines(app.scene_idx);
        let hole = app
            .game
            .as_ref()
            .map(|g| g.current_hole + 1)
            .unwrap_or(1);
        lines.push(Line::from(Span::styled(
            format!("  -- hole {hole} approach --"),
            Style::default().fg(DEEP_GREEN),
        )));
        for (i, row) in scene.iter().enumerate() {
            let fg = match i {
                0 => SKY,
                1 => SAND,
                _ => FAIRWAY,
            };
            lines.push(Line::from(Span::styled(row.clone(), Style::default().fg(fg))));
        }
    }

    let art = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(art, area);
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let scoring = app.screen == Screen::Scoring && app.game.is_some();
    // Extra room for vignette + fairway accent while scoring.
    let banner_h = if scoring { 10 } else { 6 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_h),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    banner(frame, chunks[0], app);

    match app.screen {
        Screen::MainMenu => draw_menu(frame, app, chunks[1]),
        Screen::SetupHoles => draw_setup_holes(frame, app, chunks[1]),
        Screen::SetupPlayers => draw_setup_players(frame, app, chunks[1]),
        Screen::Scoring => draw_scoring(frame, app, chunks[1]),
        Screen::Celebration => draw_celebration(frame, app, chunks[1]),
        Screen::History => draw_history(frame, app, chunks[1]),
        Screen::QuitConfirm => draw_quit(frame, app, chunks[1]),
    }

    draw_status(frame, app, chunks[2]);

    let hints = match app.screen {
        Screen::MainMenu => "↑/↓ select  ·  N/R/H/Q  ·  Enter  ·  q quit",
        Screen::SetupHoles => "←/→ or Tab toggle 9/18  ·  Enter confirm  ·  Esc menu",
        Screen::SetupPlayers => {
            "type name + Enter  ·  empty Enter = start  ·  Backspace del last  ·  Esc"
        }
        Screen::Scoring => match app.jump_phase {
            JumpPhase::Hole => "type hole # + Enter  ·  Esc cancel jump",
            JumpPhase::Player => "type player # + Enter  ·  Esc cancel jump",
            JumpPhase::Idle => "e edit/jump hole+player  ·  type score + Enter  ·  Esc menu",
        },
        Screen::Celebration => "any key / Enter → menu  ·  ASCII champagne & fireworks",
        Screen::History => "↑/↓ scroll  ·  Esc menu",
        Screen::QuitConfirm => "y quit  ·  n / Esc cancel",
    };
    frame.render_widget(footer_hints(hints), chunks[3]);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if !app.error.is_empty() {
        (
            format!("!  {}", app.error),
            Style::default().fg(FLAG).add_modifier(Modifier::BOLD),
        )
    } else if !app.status.is_empty() {
        (
            format!("*  {}", app.status),
            Style::default().fg(GREEN),
        )
    } else if app.screen == Screen::Scoring {
        (
            format!("~  {}", app.current_saying()),
            Style::default().fg(SAND),
        )
    } else {
        (
            "Fairways ahead — keep it in the short grass.".into(),
            Style::default().fg(MUTED),
        )
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(
                Style::default().fg(if app.error.is_empty() {
                    if app.screen == Screen::Scoring {
                        FAIRWAY
                    } else {
                        MUTED
                    }
                } else {
                    FLAG
                }),
            )),
        area,
    );
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.menu_items();
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let selected = i == app.menu_idx;
            let marker = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!("{marker}{label}"), style))
        })
        .collect();

    let resume_note = if app.game.is_some() {
        "\n(Unfinished round on disk — Resume available)"
    } else {
        ""
    };
    let mut body = lines;
    body.push(Line::from(""));
    body.push(Line::from(Span::styled(
        format!(
            "History entries: {}{}",
            app.history.len(),
            resume_note
        ),
        Style::default().fg(MUTED),
    )));

    frame.render_widget(
        Paragraph::new(body).block(title_block("Main Menu")),
        area,
    );
}

fn draw_setup_holes(frame: &mut Frame, app: &App, area: Rect) {
    let nine = if app.holes_choice == 9 {
        Style::default()
            .fg(Color::Black)
            .bg(GREEN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    let eighteen = if app.holes_choice == 18 {
        Style::default()
            .fg(Color::Black)
            .bg(GREEN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };

    let body = vec![
        Line::from(""),
        Line::from(Span::styled(
            "How many holes?",
            Style::default().fg(SKY).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [  9 holes  ]", nine),
            Span::raw("    "),
            Span::styled("[ 18 holes ]", eighteen),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Play Nine card game: usually 9; 18 = double round.",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(body).block(title_block("Setup · Holes")),
        area,
    );
}

fn draw_setup_players(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Players for a {}-hole round:", app.holes_choice),
            Style::default().fg(SKY),
        )),
        Line::from(""),
    ];
    if app.pending_names.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none yet)",
            Style::default().fg(MUTED),
        )));
    } else {
        for (i, n) in app.pending_names.iter().enumerate() {
            lines.push(Line::from(Span::styled(
                format!("  {}. {}", i + 1, n),
                Style::default().fg(GREEN),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Name> ", Style::default().fg(SAND).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}_", app.name_buf),
            Style::default().fg(Color::White),
        ),
    ]));

    frame.render_widget(
        Paragraph::new(lines).block(title_block("Setup · Players")),
        area,
    );
}

fn draw_scoring(frame: &mut Frame, app: &App, area: Rect) {
    let Some(game) = app.game.as_ref() else {
        frame.render_widget(
            Paragraph::new("No active game.").block(title_block("Scoring")),
            area,
        );
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
        .split(area);

    let hole_num = game.current_hole + 1;
    let player = game
        .players
        .get(game.current_player)
        .map(|p| p.name.as_str())
        .unwrap_or("?");

    let prompt_lines = match app.jump_phase {
        JumpPhase::Hole => vec![
            Line::from(Span::styled(
                format!("Jump — hole number (1..={}):", game.holes),
                Style::default().fg(SKY).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "Hole> ",
                    Style::default().fg(SAND).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}_", app.jump_buf),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(Span::styled(
                "Enter to confirm, Esc to cancel.",
                Style::default().fg(MUTED),
            )),
        ],
        JumpPhase::Player => vec![
            Line::from(Span::styled(
                format!(
                    "Jump hole {} — player number (1..={}):",
                    app.jump_hole,
                    game.players.len()
                ),
                Style::default().fg(SKY).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "Player> ",
                    Style::default().fg(SAND).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}_", app.jump_buf),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(Span::styled(
                "Enter to jump, Esc to cancel.",
                Style::default().fg(MUTED),
            )),
        ],
        JumpPhase::Idle => vec![
            Line::from(vec![
                Span::styled("Hole ", Style::default().fg(MUTED)),
                Span::styled(
                    format!("{hole_num}"),
                    Style::default()
                        .fg(FLAG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" / {}", game.holes),
                    Style::default().fg(MUTED),
                ),
                Span::styled("   ·   scoring: ", Style::default().fg(MUTED)),
                Span::styled(
                    player,
                    Style::default()
                        .fg(SKY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Score> ",
                    Style::default().fg(SAND).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}_", app.score_buf),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(Span::styled(
                "Integers only (-20..40). Lowest cumulative total wins.",
                Style::default().fg(MUTED),
            )),
        ],
    };

    let prompt = Paragraph::new(prompt_lines).block(title_block("On the tee"));
    frame.render_widget(prompt, chunks[0]);

    let leaders = game.leader_indices();

    let header = Row::new(
        std::iter::once("Player".to_string())
            .chain((1..=game.holes).map(|h| format!("H{h}")))
            .chain(std::iter::once("Tot".into()))
            .map(|s| Span::styled(s, Style::default().fg(SAND).add_modifier(Modifier::BOLD))),
    );

    let rows = game.players.iter().enumerate().map(|(pi, p)| {
        let mut cells: Vec<Span> = Vec::new();
        let is_leader = leaders.contains(&pi);
        let name_label = if is_leader {
            format!("▲ {}", p.name)
        } else {
            format!("  {}", p.name)
        };
        let name_style = if pi == game.current_player {
            Style::default()
                .fg(Color::Black)
                .bg(GREEN)
                .add_modifier(Modifier::BOLD)
        } else if is_leader {
            Style::default().fg(SAND).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        cells.push(Span::styled(name_label, name_style));
        for (hi, sc) in p.scores.iter().enumerate() {
            let txt = match sc {
                Some(v) => v.to_string(),
                None if hi == game.current_hole && pi == game.current_player => "·".into(),
                None => "-".into(),
            };
            let st = if hi == game.current_hole && pi == game.current_player {
                Style::default().fg(FLAG)
            } else {
                Style::default().fg(GREEN)
            };
            cells.push(Span::styled(txt, st));
        }
        cells.push(Span::styled(
            p.total().to_string(),
            Style::default()
                .fg(SKY)
                .add_modifier(Modifier::BOLD),
        ));
        Row::new(cells)
    });

    let widths: Vec<Constraint> = std::iter::once(Constraint::Length(14))
        .chain((0..game.holes).map(|_| Constraint::Length(4)))
        .chain(std::iter::once(Constraint::Length(5)))
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(title_block("Scorecard"))
        .column_spacing(0);

    frame.render_widget(table, chunks[1]);
}

fn draw_celebration(frame: &mut Frame, app: &App, area: Rect) {
    let winners = app.last_winners();
    let tick = app.celebration_tick as usize;

    let name_banner = if winners.is_empty() {
        "***  CHAMPION OF THE FAIRWAY  ***".to_string()
    } else if winners.len() == 1 {
        format!("***  {} WINS THE ROUND  ***", winners[0].to_uppercase())
    } else {
        format!("***  TIE: {}  ***", winners.join(" & ").to_uppercase())
    };

    let glasses = champagne_frame(tick);
    let fireworks = fireworks_frame(tick);
    let rain1 = confetti_rain(tick, 36);
    let rain2 = confetti_rain(tick.wrapping_add(11), 36);
    let rain3 = confetti_rain(tick.wrapping_add(23), 36);

    let results = app
        .history
        .first()
        .map(|h| {
            h.results
                .iter()
                .map(|(n, t)| format!("{n}: {t}"))
                .collect::<Vec<_>>()
                .join("   ")
        })
        .unwrap_or_default();

    let body = vec![
        Line::from(Span::styled(rain1, Style::default().fg(SAND))),
        Line::from(Span::styled(fireworks.clone(), Style::default().fg(SKY))),
        Line::from(""),
        Line::from(Span::styled(
            name_banner,
            Style::default()
                .fg(Color::Black)
                .bg(GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(glasses, Style::default().fg(SAND).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            "  pop!  *clink*  19th hole open  *clink*  pop!  ",
            Style::default().fg(MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(fireworks, Style::default().fg(FLAG))),
        Line::from(Span::styled(rain2, Style::default().fg(FOAM))),
        Line::from(""),
        Line::from(Span::styled(
            "Final totals",
            Style::default().fg(SAND).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(results, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(Span::styled(rain3, Style::default().fg(SKY))),
    ];

    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .block(title_block("19th Hole · Celebration")),
        area,
    );
}

fn champagne_frame(tick: usize) -> String {
    // Alternating toast poses — ASCII only.
    let frames = [
        r#"  \o/   † †   \o/   cheers!   \o/   † †   \o/  "#,
        r#"  /o\   † †   /o\   cheers!   /o\   † †   /o\  "#,
        r#"  \o/    Y     \o/  cheers!   \o/    Y     \o/  "#,
        r#"  |o|   † †   |o|   cheers!   |o|   † †   |o|  "#,
    ];
    frames[tick % frames.len()].to_string()
}

fn fireworks_frame(tick: usize) -> String {
    let frames = [
        r#"  *  .  *  .  *    ·  *  ·    *  .  *  .  *  "#,
        r#"  .  *  ·  *  .    *  .  *    .  *  ·  *  .  "#,
        r#"  ·  *  *  ·  *    *  ·  *    ·  *  *  ·  *  "#,
        r#"  *  ·  .  *  *    .  *  .    *  ·  .  *  *  "#,
    ];
    frames[tick % frames.len()].to_string()
}

fn confetti_rain(tick: usize, cols: usize) -> String {
    const CHARS: &[char] = &['*', '.', 'o', '+', '~', '`', '-', '=', ':', '"', ','];
    let mut s = String::from("  ");
    for i in 0..cols {
        // Cascading phase so rows look like falling confetti.
        let phase = tick.wrapping_add(i * 5).wrapping_add((i % 7) * 3);
        let c = CHARS[phase % CHARS.len()];
        // Sparse gaps for denser-but-readable rain.
        if (phase / CHARS.len()) % 5 == 0 {
            s.push(' ');
        } else {
            s.push(c);
        }
        s.push(' ');
    }
    s
}

fn draw_history(frame: &mut Frame, app: &App, area: Rect) {
    if app.history.is_empty() {
        frame.render_widget(
            Paragraph::new("No finished rounds yet. Play a game!")
                .style(Style::default().fg(MUTED))
                .block(title_block("History")),
            area,
        );
        return;
    }

    let visible = area.height.saturating_sub(2) as usize;
    let start = app.history_scroll.min(app.history.len().saturating_sub(1));
    let mut lines = Vec::new();
    for (i, entry) in app.history.iter().enumerate().skip(start).take(visible) {
        let winners = entry.winners.join(", ");
        let scores = entry
            .results
            .iter()
            .map(|(n, t)| format!("{n}={t}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(Span::styled(
            format!(
                "{}. [{}] {}h  winner(s): {}  |  {}",
                i + 1,
                entry.finished_at,
                entry.holes,
                winners,
                scores
            ),
            Style::default().fg(if i == start { GREEN } else { Color::White }),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines).block(title_block("History (summarized)")),
        area,
    );
}

fn draw_quit(frame: &mut Frame, _app: &App, area: Rect) {
    let popup = centered_rect(50, 30, area);
    frame.render_widget(Clear, popup);
    let body = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Walk off the course?",
            Style::default()
                .fg(FLAG)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press y to quit, n to stay."),
        Line::from(Span::styled(
            "(Unfinished game is saved to data/play_nine.json)",
            Style::default().fg(MUTED),
        )),
    ])
    .alignment(Alignment::Center)
    .block(title_block("Quit"));
    frame.render_widget(body, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
