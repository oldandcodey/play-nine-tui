//! Application state machine for the Play Nine scorekeeper.

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::persist::{self, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    SetupHoles,
    SetupPlayers,
    Scoring,
    Celebration,
    History,
    QuitConfirm,
}

/// Jump/edit flow while on the Scoring screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JumpPhase {
    #[default]
    Idle,
    Hole,
    Player,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    /// One entry per hole; `None` means not yet entered.
    pub scores: Vec<Option<i32>>,
}

impl Player {
    pub fn new(name: String, holes: usize) -> Self {
        Self {
            name,
            scores: vec![None; holes],
        }
    }

    pub fn total(&self) -> i32 {
        self.scores.iter().flatten().copied().sum()
    }

    pub fn has_any_score(&self) -> bool {
        self.scores.iter().any(|s| s.is_some())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub holes: u8,
    pub players: Vec<Player>,
    /// 0-based hole index currently being scored.
    pub current_hole: usize,
    /// 0-based player index whose score is being entered.
    pub current_player: usize,
    pub finished: bool,
}

impl Game {
    pub fn new(holes: u8, names: Vec<String>) -> Self {
        let n = holes as usize;
        Self {
            holes,
            players: names.into_iter().map(|nme| Player::new(nme, n)).collect(),
            current_hole: 0,
            current_player: 0,
            finished: false,
        }
    }

    pub fn set_current_score(&mut self, score: i32) {
        if let Some(p) = self.players.get_mut(self.current_player) {
            if let Some(slot) = p.scores.get_mut(self.current_hole) {
                *slot = Some(score);
            }
        }
    }

    pub fn current_score(&self) -> Option<i32> {
        self.players
            .get(self.current_player)
            .and_then(|p| p.scores.get(self.current_hole).copied())
            .flatten()
    }

    /// Advance to next player / hole. Returns true when the round is complete.
    pub fn advance(&mut self) -> bool {
        if self.finished {
            return true;
        }
        self.current_player += 1;
        if self.current_player >= self.players.len() {
            self.current_player = 0;
            self.current_hole += 1;
            if self.current_hole >= self.holes as usize {
                self.finished = true;
                return true;
            }
        }
        false
    }

    /// Hole-major, player-minor scan for the first empty slot.
    /// Returns true when no empty slots remain (round complete).
    pub fn seek_next_empty(&mut self) -> bool {
        for h in 0..self.holes as usize {
            for p in 0..self.players.len() {
                if self.players[p].scores[h].is_none() {
                    self.current_hole = h;
                    self.current_player = p;
                    self.finished = false;
                    return false;
                }
            }
        }
        self.finished = true;
        true
    }

    /// Player indices tied for lowest total among those with ≥1 entered score.
    pub fn leader_indices(&self) -> Vec<usize> {
        let scored: Vec<(usize, i32)> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.has_any_score())
            .map(|(i, p)| (i, p.total()))
            .collect();
        let Some(best) = scored.iter().map(|(_, t)| *t).min() else {
            return vec![];
        };
        scored
            .into_iter()
            .filter(|(_, t)| *t == best)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn winner_names(&self) -> Vec<String> {
        if self.players.is_empty() {
            return vec![];
        }
        let best = self.players.iter().map(|p| p.total()).min().unwrap_or(0);
        self.players
            .iter()
            .filter(|p| p.total() == best)
            .map(|p| p.name.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub finished_at: String,
    pub holes: u8,
    pub results: Vec<(String, i32)>,
    pub winners: Vec<String>,
}

#[derive(Debug)]
pub struct App {
    pub screen: Screen,
    pub menu_idx: usize,
    pub holes_choice: u8, // 9 or 18
    pub name_buf: String,
    pub score_buf: String,
    pub pending_names: Vec<String>,
    pub game: Option<Game>,
    pub history: Vec<HistoryEntry>,
    pub status: String,
    pub error: String,
    pub celebration_tick: u16,
    pub history_scroll: usize,
    pub jump_phase: JumpPhase,
    pub jump_buf: String,
    /// Pending 1-based hole chosen during JumpPhase::Hole → Player.
    pub jump_hole: usize,
    /// True after a jump until the next score submit (then seek empty).
    pub jumped: bool,
    /// Index into [`crate::sayings::SAYINGS`]; refreshed on hole change.
    pub saying_idx: usize,
    /// Last `current_hole` we flavored (None = never).
    pub last_saying_hole: Option<usize>,
}

impl App {
    pub fn boot() -> Result<Self, String> {
        let store = persist::load()?;
        let mut app = Self {
            screen: Screen::MainMenu,
            menu_idx: 0,
            holes_choice: 9,
            name_buf: String::new(),
            score_buf: String::new(),
            pending_names: Vec::new(),
            game: store.current_game,
            history: store.history,
            status: String::new(),
            error: String::new(),
            celebration_tick: 0,
            history_scroll: 0,
            jump_phase: JumpPhase::Idle,
            jump_buf: String::new(),
            jump_hole: 1,
            jumped: false,
            saying_idx: 0,
            last_saying_hole: None,
        };
        app.refresh_saying();
        if app.game.as_ref().is_some_and(|g| g.finished) {
            // Finished games belong in history only; clear stale current.
            app.game = None;
            let _ = app.persist();
        }
        Ok(app)
    }

    pub fn persist(&self) -> Result<(), String> {
        let store = Store {
            current_game: self.game.clone(),
            history: self.history.clone(),
        };
        persist::save(&store)
    }

    pub fn clear_msgs(&mut self) {
        self.error.clear();
        self.status.clear();
    }

    fn entropy_seed() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42)
    }

    /// Pick a new funny golf saying (not every keypress — call on hole change).
    pub fn refresh_saying(&mut self) {
        let seed = Self::entropy_seed() ^ ((self.saying_idx as u64) << 9);
        self.saying_idx = crate::sayings::next_saying_idx(self.saying_idx, seed);
    }

    /// Re-roll saying when `current_hole` changes (or first entry).
    pub fn on_hole_context_changed(&mut self) {
        let hole = self.game.as_ref().map(|g| g.current_hole);
        if hole != self.last_saying_hole {
            self.last_saying_hole = hole;
            self.refresh_saying();
        }
    }

    pub fn current_saying(&self) -> &'static str {
        crate::sayings::saying_at(self.saying_idx)
    }

    // --- Main menu ---

    pub fn menu_items(&self) -> Vec<&'static str> {
        let mut items = vec!["[N] New game"];
        if self.game.is_some() {
            items.push("[R] Resume game");
        }
        items.push("[H] History");
        items.push("[Q] Quit");
        items
    }

    pub fn menu_up(&mut self) {
        let n = self.menu_items().len();
        if n == 0 {
            return;
        }
        self.menu_idx = (self.menu_idx + n - 1) % n;
    }

    pub fn menu_down(&mut self) {
        let n = self.menu_items().len();
        if n == 0 {
            return;
        }
        self.menu_idx = (self.menu_idx + 1) % n;
    }

    pub fn menu_activate_by_letter(&mut self, c: char) {
        let needle = format!("[{}]", c.to_ascii_uppercase());
        let items = self.menu_items();
        if let Some(idx) = items.iter().position(|label| label.contains(&needle)) {
            self.menu_idx = idx;
            self.menu_select();
        }
    }

    pub fn menu_select(&mut self) {
        self.clear_msgs();
        let items = self.menu_items();
        let Some(&label) = items.get(self.menu_idx) else {
            return;
        };
        if label.contains("New game") {
            self.holes_choice = 9;
            self.pending_names.clear();
            self.name_buf.clear();
            self.screen = Screen::SetupHoles;
        } else if label.contains("Resume") {
            if self.game.is_some() {
                self.score_buf.clear();
                self.jump_phase = JumpPhase::Idle;
                self.jumped = false;
                self.last_saying_hole = None;
                self.screen = Screen::Scoring;
                self.on_hole_context_changed();
            }
        } else if label.contains("History") {
            self.history_scroll = 0;
            self.screen = Screen::History;
        } else if label.contains("Quit") {
            self.screen = Screen::QuitConfirm;
        }
    }

    // --- Setup ---

    pub fn toggle_holes(&mut self) {
        self.holes_choice = if self.holes_choice == 9 { 18 } else { 9 };
    }

    pub fn confirm_holes(&mut self) {
        self.clear_msgs();
        self.pending_names.clear();
        self.name_buf.clear();
        self.screen = Screen::SetupPlayers;
    }

    pub fn add_player_name(&mut self) {
        self.clear_msgs();
        let name = self.name_buf.trim().to_string();
        if name.is_empty() {
            self.error = "Name cannot be empty.".into();
            return;
        }
        if name.len() > 24 {
            self.error = "Keep names to 24 characters or fewer.".into();
            return;
        }
        if self
            .pending_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&name))
        {
            self.error = format!("\"{name}\" is already on the card.");
            return;
        }
        if self.pending_names.len() >= 8 {
            self.error = "Max 8 players for this scorekeeper.".into();
            return;
        }
        self.pending_names.push(name);
        self.name_buf.clear();
        self.status = format!(
            "Added. {} player(s). Enter more, or press Enter on empty to start.",
            self.pending_names.len()
        );
    }

    pub fn start_game_from_names(&mut self) {
        self.clear_msgs();
        // If buffer has a name, add it first.
        if !self.name_buf.trim().is_empty() {
            self.add_player_name();
            if !self.error.is_empty() {
                return;
            }
        }
        if self.pending_names.is_empty() {
            self.error = "Add at least one player.".into();
            return;
        }
        self.game = Some(Game::new(self.holes_choice, self.pending_names.clone()));
        self.score_buf.clear();
        self.jump_phase = JumpPhase::Idle;
        self.jumped = false;
        self.last_saying_hole = None;
        self.screen = Screen::Scoring;
        self.on_hole_context_changed();
        if let Err(e) = self.persist() {
            self.error = e;
        } else {
            self.status = "Game started — lowest total wins (golf).".into();
        }
    }

    pub fn remove_last_pending_name(&mut self) {
        self.clear_msgs();
        if self.pending_names.pop().is_some() {
            self.status = "Removed last player.".into();
        }
    }

    // --- Scoring / jump ---

    pub fn begin_jump(&mut self) {
        self.clear_msgs();
        self.jump_phase = JumpPhase::Hole;
        self.jump_buf.clear();
        self.status = "Jump: enter hole number (1..=holes), then Enter.".into();
    }

    pub fn cancel_jump(&mut self) {
        self.jump_phase = JumpPhase::Idle;
        self.jump_buf.clear();
        self.clear_msgs();
    }

    pub fn submit_jump_hole(&mut self) {
        self.clear_msgs();
        let Some(game) = self.game.as_ref() else {
            self.error = "No active game.".into();
            self.cancel_jump();
            return;
        };
        let raw = self.jump_buf.trim();
        let Ok(hole) = raw.parse::<usize>() else {
            self.error = format!("\"{raw}\" is not a hole number.");
            return;
        };
        if !(1..=game.holes as usize).contains(&hole) {
            self.error = format!("Hole must be 1..={}.", game.holes);
            return;
        }
        self.jump_hole = hole;
        self.jump_buf.clear();
        self.jump_phase = JumpPhase::Player;
        self.status = format!(
            "Jump hole {hole}: enter player number (1..={}), then Enter.",
            game.players.len()
        );
    }

    pub fn submit_jump_player(&mut self) {
        self.clear_msgs();
        let Some(game) = self.game.as_mut() else {
            self.error = "No active game.".into();
            self.cancel_jump();
            return;
        };
        let raw = self.jump_buf.trim();
        let Ok(player) = raw.parse::<usize>() else {
            self.error = format!("\"{raw}\" is not a player number.");
            return;
        };
        if !(1..=game.players.len()).contains(&player) {
            self.error = format!("Player must be 1..={}.", game.players.len());
            return;
        }
        game.current_hole = self.jump_hole - 1;
        game.current_player = player - 1;
        let existing = game.current_score();
        self.score_buf = existing.map(|s| s.to_string()).unwrap_or_default();
        self.jumped = true;
        self.jump_phase = JumpPhase::Idle;
        self.jump_buf.clear();
        let pname = game
            .players
            .get(game.current_player)
            .map(|p| p.name.as_str())
            .unwrap_or("?");
        self.status = format!(
            "Editing hole {} / {} — type new score + Enter.",
            self.jump_hole, pname
        );
        self.on_hole_context_changed();
    }

    pub fn submit_score(&mut self) {
        self.clear_msgs();
        let raw = self.score_buf.trim();
        if raw.is_empty() {
            self.error = "Enter an integer score for this hole.".into();
            return;
        }
        let Ok(score) = raw.parse::<i32>() else {
            self.error = format!("\"{raw}\" is not an integer. Try again.");
            return;
        };
        // Play Nine hole scores are typically small; allow a sane range.
        if !(-20..=40).contains(&score) {
            self.error = "Score out of range (−20…40). Check and retry.".into();
            return;
        }
        let Some(game) = self.game.as_mut() else {
            self.error = "No active game.".into();
            return;
        };
        let was_jumped = self.jumped;
        self.jumped = false;
        game.set_current_score(score);
        self.score_buf.clear();
        let done = if was_jumped {
            game.seek_next_empty()
        } else {
            game.advance()
        };
        if done {
            self.finish_game();
        } else {
            self.on_hole_context_changed();
            if let Err(e) = self.persist() {
                self.error = e;
            }
        }
    }

    fn finish_game(&mut self) {
        let Some(game) = self.game.clone() else {
            return;
        };
        let winners = game.winner_names();
        let results: Vec<(String, i32)> = game
            .players
            .iter()
            .map(|p| (p.name.clone(), p.total()))
            .collect();
        let entry = HistoryEntry {
            finished_at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            holes: game.holes,
            results,
            winners: winners.clone(),
        };
        self.history.insert(0, entry);
        // Cap history so the JSON stays small.
        if self.history.len() > 50 {
            self.history.truncate(50);
        }
        self.game = None;
        self.celebration_tick = 0;
        self.jump_phase = JumpPhase::Idle;
        self.jumped = false;
        self.screen = Screen::Celebration;
        let _ = self.persist();
        // Re-attach a finished snapshot only for celebration rendering via last history.
        // Celebration reads `last_winners` / history[0].
        self.status = if winners.len() == 1 {
            format!("Winner: {}!", winners[0])
        } else {
            format!("Tie: {}!", winners.join(" & "))
        };
    }

    pub fn last_winners(&self) -> Vec<String> {
        self.history
            .first()
            .map(|h| h.winners.clone())
            .unwrap_or_default()
    }

    pub fn tick_celebration(&mut self) {
        self.celebration_tick = self.celebration_tick.wrapping_add(1);
    }

    pub fn leave_celebration(&mut self) {
        self.clear_msgs();
        self.menu_idx = 0;
        self.screen = Screen::MainMenu;
    }

    pub fn back_to_menu(&mut self) {
        self.clear_msgs();
        self.menu_idx = 0;
        self.jump_phase = JumpPhase::Idle;
        self.jump_buf.clear();
        self.jumped = false;
        self.screen = Screen::MainMenu;
        let _ = self.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_hole_two_players_finish() {
        let mut g = Game::new(9, vec!["Ada".into(), "Ben".into()]);
        for hole in 0..9 {
            for pl in 0..2 {
                assert_eq!(g.current_hole, hole);
                assert_eq!(g.current_player, pl);
                g.set_current_score(if pl == 0 { 1 } else { 2 });
                let done = g.advance();
                if hole == 8 && pl == 1 {
                    assert!(done);
                    assert!(g.finished);
                } else {
                    assert!(!done);
                }
            }
        }
        assert_eq!(g.players[0].total(), 9);
        assert_eq!(g.players[1].total(), 18);
        assert_eq!(g.winner_names(), vec!["Ada".to_string()]);
    }

    #[test]
    fn tie_winners() {
        let mut g = Game::new(9, vec!["A".into(), "B".into()]);
        for _ in 0..18 {
            g.set_current_score(3);
            g.advance();
        }
        assert_eq!(g.winner_names().len(), 2);
    }

    #[test]
    fn seek_next_empty_skips_filled() {
        let mut g = Game::new(3, vec!["A".into(), "B".into()]);
        // Fill hole 0 both players, leave hole 1 empty.
        g.current_hole = 0;
        g.current_player = 0;
        g.set_current_score(1);
        g.advance();
        g.set_current_score(2);
        g.advance();
        // Jump-correct hole 0 player 0, then seek.
        g.current_hole = 0;
        g.current_player = 0;
        g.set_current_score(0);
        assert!(!g.seek_next_empty());
        assert_eq!(g.current_hole, 1);
        assert_eq!(g.current_player, 0);
    }

    #[test]
    fn seek_next_empty_finishes_when_full() {
        let mut g = Game::new(2, vec!["A".into()]);
        g.current_hole = 0;
        g.current_player = 0;
        g.set_current_score(1);
        g.advance();
        g.set_current_score(2);
        assert!(g.seek_next_empty());
        assert!(g.finished);
    }

    #[test]
    fn leaders_only_among_scored() {
        let mut g = Game::new(2, vec!["A".into(), "B".into(), "C".into()]);
        assert!(g.leader_indices().is_empty());
        g.players[0].scores[0] = Some(5);
        g.players[1].scores[0] = Some(3);
        // C has no scores — not a leader
        assert_eq!(g.leader_indices(), vec![1]);
        g.players[0].scores[0] = Some(3);
        assert_eq!(g.leader_indices(), vec![0, 1]);
    }

    #[test]
    fn menu_items_show_shortcuts() {
        let app = App {
            screen: Screen::MainMenu,
            menu_idx: 0,
            holes_choice: 9,
            name_buf: String::new(),
            score_buf: String::new(),
            pending_names: Vec::new(),
            game: None,
            history: Vec::new(),
            status: String::new(),
            error: String::new(),
            celebration_tick: 0,
            history_scroll: 0,
            jump_phase: JumpPhase::Idle,
            jump_buf: String::new(),
            jump_hole: 1,
            jumped: false,
            saying_idx: 0,
            last_saying_hole: None,
        };
        let items = app.menu_items();
        assert!(items.iter().any(|i| i.contains("[N]")));
        assert!(items.iter().any(|i| i.contains("[H]")));
        assert!(items.iter().any(|i| i.contains("[Q]")));
        assert!(!items.iter().any(|i| i.contains("[R]")));
    }

    #[test]
    fn saying_refreshes_when_hole_changes() {
        let mut app = App {
            screen: Screen::Scoring,
            menu_idx: 0,
            holes_choice: 9,
            name_buf: String::new(),
            score_buf: String::new(),
            pending_names: Vec::new(),
            game: Some(Game::new(9, vec!["A".into(), "B".into()])),
            history: Vec::new(),
            status: String::new(),
            error: String::new(),
            celebration_tick: 0,
            history_scroll: 0,
            jump_phase: JumpPhase::Idle,
            jump_buf: String::new(),
            jump_hole: 1,
            jumped: false,
            saying_idx: 0,
            last_saying_hole: None,
        };
        app.on_hole_context_changed();
        let first = app.saying_idx;
        // Same hole: no change
        app.on_hole_context_changed();
        assert_eq!(app.saying_idx, first);
        // Advance hole
        if let Some(g) = app.game.as_mut() {
            g.current_hole = 1;
        }
        app.on_hole_context_changed();
        // Index may coincidentally match (unlikely); saying should move.
        // Force another hole to be confident we don't panic and path runs.
        if let Some(g) = app.game.as_mut() {
            g.current_hole = 2;
        }
        app.on_hole_context_changed();
        assert_eq!(app.last_saying_hole, Some(2));
        assert!(!app.current_saying().is_empty());
    }
}
