use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::game::Game;
use crate::settings::Settings;

#[derive(Debug, Clone, Default)]
pub struct GameStatus {
    pub enabled: bool,
    pub running: bool,
    pub accepted: u32,
    pub last_accepted: Option<SystemTime>,
}

#[derive(Default)]
pub struct AcceptState {
    games: Mutex<HashMap<Game, GameStatus>>,
}

impl AcceptState {
    pub fn apply_settings(&self, settings: &Settings) {
        let mut games = self.games.lock().unwrap();
        for game in Game::ALL {
            games.entry(game).or_default().enabled = settings.enabled(game);
        }
    }

    pub fn set_enabled(&self, game: Game, enabled: bool) {
        self.games.lock().unwrap().entry(game).or_default().enabled = enabled;
    }

    pub fn is_enabled(&self, game: Game) -> bool {
        self.status(game).enabled
    }

    pub fn set_running(&self, game: Game, running: bool) {
        self.games.lock().unwrap().entry(game).or_default().running = running;
    }

    pub fn record_accept(&self, game: Game) {
        let mut games = self.games.lock().unwrap();
        let status = games.entry(game).or_default();
        status.accepted += 1;
        status.last_accepted = Some(SystemTime::now());
    }

    pub fn status(&self, game: Game) -> GameStatus {
        self.games
            .lock()
            .unwrap()
            .get(&game)
            .cloned()
            .unwrap_or_default()
    }
}
