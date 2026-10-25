use std::time::SystemTime;

use crate::game::Game;
use crate::state::AcceptState;

pub fn status(state: &AcceptState) -> String {
    Game::ALL
        .iter()
        .map(|game| {
            let status = state.status(*game);
            let mut line = format!(
                "{}: {}, {}, accepted {}",
                game.title(),
                if status.enabled { "on" } else { "off" },
                if status.running { "running" } else { "not running" },
                status.accepted
            );
            if let Some(seconds) = status
                .last_accepted
                .and_then(|at| SystemTime::now().duration_since(at).ok())
                .map(|age| age.as_secs())
            {
                line.push_str(&format!(" (last {seconds}s ago)"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}
