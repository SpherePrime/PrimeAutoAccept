use std::time::SystemTime;

use astra_plugin_sdk::prelude::*;

use crate::game::Game;
use crate::state::AcceptState;

pub fn status(state: &AcceptState, i18n: &I18n) -> String {
    Game::ALL
        .iter()
        .map(|game| {
            let status = state.status(*game);
            let enabled = i18n.t(if status.enabled { "status.on" } else { "status.off" });
            let running = i18n.t(if status.running {
                "status.running"
            } else {
                "status.not_running"
            });
            let accepted = status.accepted.to_string();
            let mut line = i18n.tn(
                "status.line",
                status.accepted as i64,
                &[
                    ("game", game.title()),
                    ("enabled", &enabled),
                    ("running", &running),
                    ("n", &accepted),
                ],
            );
            if let Some(seconds) = status
                .last_accepted
                .and_then(|at| SystemTime::now().duration_since(at).ok())
                .map(|age| age.as_secs() as i64)
            {
                line.push_str(&i18n.tn("status.last", seconds, &[("n", &seconds.to_string())]));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}
