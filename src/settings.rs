use astra_plugin_sdk::prelude::*;

use crate::game::Game;

#[astra::config]
#[derive(Debug, Clone)]
pub struct Settings {
    /// Accept League of Legends ready checks
    #[schemars(title = "League of Legends")]
    pub lol: bool,
    /// Accept Dota 2 matches
    #[schemars(title = "Dota 2")]
    pub dota2: bool,
    /// Accept Counter-Strike 2 matches
    #[schemars(title = "Counter-Strike 2")]
    pub cs2: bool,
    /// Wait this many milliseconds before accepting
    #[schemars(title = "Accept delay, ms", range(max = 10000))]
    pub accept_delay_ms: u64,
    /// Bring Dota 2 or CS2 to the front to press Accept
    #[schemars(title = "Focus the game window")]
    pub focus_game: bool,
    /// League of Legends folder, if the client is not found automatically
    #[schemars(title = "League of Legends folder")]
    pub lol_install_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lol: true,
            dota2: true,
            cs2: true,
            accept_delay_ms: 1500,
            focus_game: true,
            lol_install_dir: String::new(),
        }
    }
}

impl Settings {
    pub fn enabled(&self, game: Game) -> bool {
        match game {
            Game::LeagueOfLegends => self.lol,
            Game::Dota2 => self.dota2,
            Game::Cs2 => self.cs2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_schema_matches_settings() {
        let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml")).unwrap();
        let schema = <Settings as PluginConfig>::json_schema();
        assert!(manifest.contains(&format!("schema = {schema:?}")), "update [config] schema in plugin.toml");
    }
}
