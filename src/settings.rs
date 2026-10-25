use astra_plugin_sdk::prelude::*;

use crate::game::Game;

#[astra::config]
#[derive(Debug, Clone)]
pub struct Settings {
    #[schemars(title = "$config.lol.title", description = "$config.lol.description")]
    pub lol: bool,
    #[schemars(title = "$config.dota2.title", description = "$config.dota2.description")]
    pub dota2: bool,
    #[schemars(title = "$config.cs2.title", description = "$config.cs2.description")]
    pub cs2: bool,
    #[schemars(title = "$config.accept_delay_ms.title", description = "$config.accept_delay_ms.description", range(max = 10000))]
    pub accept_delay_ms: u64,
    #[schemars(title = "$config.focus_game.title", description = "$config.focus_game.description")]
    pub focus_game: bool,
    #[schemars(title = "$config.lol_install_dir.title", description = "$config.lol_install_dir.description")]
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
