mod capture;
mod detector;
mod game;
mod input;
mod lcu;
mod process;
mod report;
mod settings;
mod state;
mod watcher;
mod window;

use std::sync::Arc;

use astra_plugin_sdk::prelude::*;

use game::Game;
use settings::Settings;
use state::AcceptState;
use watcher::{MATCH_ACCEPTED, MATCH_FOUND, Watcher};

const FOUND_ICON: &str = "<svg viewBox='0 0 24 24'><circle cx='12' cy='12' r='8' fill='none' stroke='currentColor' stroke-width='2'/><path d='M12 7v5l3 2' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'/></svg>";
const ACCEPTED_ICON: &str = "<svg viewBox='0 0 24 24'><path d='M5 12.5 10 17l9-10' fill='none' stroke='currentColor' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round'/></svg>";

#[derive(Default)]
struct PrimeAutoAccept {
    settings: Arc<Config<Settings>>,
    state: Arc<AcceptState>,
}

#[astra::args]
struct Toggle {
    /// Game to change: "lol", "dota2", "cs2" or "all"
    game: String,
    /// true turns auto-accept on, false turns it off
    enabled: bool,
}

#[astra::plugin(capabilities = "tools, triggers")]
impl PrimeAutoAccept {
    /// Show whether match auto-accept is on for League of Legends, Dota 2 and CS2, which of those games are running, and how many matches were accepted.
    #[tool]
    async fn auto_accept_status(&self, ctx: &PluginContext) -> Result<String, ToolError> {
        Ok(report::status(&self.state, ctx.i18n()))
    }

    /// Turn match auto-accept on or off for League of Legends, Dota 2, CS2 or all of them until Astra restarts.
    #[tool]
    async fn set_auto_accept(&self, ctx: &PluginContext, args: Toggle) -> Result<String, ToolError> {
        let games: Vec<Game> = if args.game.trim().eq_ignore_ascii_case("all") {
            Game::ALL.to_vec()
        } else {
            vec![Game::parse(&args.game).ok_or_else(|| {
                ToolError::BadArguments(ctx.i18n().ta("error.unknown_game", &[("game", &args.game)]))
            })?]
        };
        for game in &games {
            self.state.set_enabled(*game, args.enabled);
        }
        Ok(report::status(&self.state, ctx.i18n()))
    }

    #[hook]
    async fn trigger_types(&self) -> Vec<TriggerTypeDef> {
        vec![
            TriggerTypeDef::new(MATCH_FOUND, key("trigger.match_found.label")).with_icon_svg(FOUND_ICON),
            TriggerTypeDef::new(MATCH_ACCEPTED, key("trigger.match_accepted.label"))
                .with_icon_svg(ACCEPTED_ICON),
        ]
    }

    #[hook]
    async fn on_config(&self, _ctx: &PluginContext, config: Settings) {
        self.state.apply_settings(&config);
        self.settings.store(config);
    }

    #[hook]
    async fn on_start(&self, ctx: &PluginContext) -> anyhow::Result<()> {
        self.state.apply_settings(&self.settings.load());
        let watcher = Watcher::new(ctx.clone(), self.settings.clone(), self.state.clone());
        tokio::spawn(watcher.run());
        Ok(())
    }
}

astra::main!(PrimeAutoAccept::default());

#[cfg(test)]
mod tests {
    use super::*;
    use astra_plugin_sdk::testing::Harness;

    #[tokio::test]
    async fn toggles_and_reports() {
        let h = Harness::new(PrimeAutoAccept::default())
            .with_config(json!({ "cs2": false }))
            .start()
            .await
            .expect("the plugin started");

        let status = h.call_tool("auto_accept_status", json!({})).await.unwrap();
        assert!(status.contains("Counter-Strike 2: off, not running, 0 matches accepted"), "{status}");

        let status = h
            .call_tool("set_auto_accept", json!({ "game": "cs2", "enabled": true }))
            .await
            .unwrap();
        assert!(status.contains("Counter-Strike 2: on"), "{status}");

        assert!(
            h.call_tool("set_auto_accept", json!({ "game": "valorant", "enabled": true }))
                .await
                .is_err()
        );
        assert!(h.health().await.0);
    }
}
