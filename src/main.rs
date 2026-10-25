#[cfg(windows)]
mod capture;
mod detector;
mod game;
#[cfg(windows)]
mod input;
#[cfg(windows)]
mod lcu;
#[cfg(windows)]
mod process;
mod report;
mod settings;
mod state;
mod triggers;
#[cfg(windows)]
mod watcher;
#[cfg(windows)]
mod window;

use std::sync::Arc;

use astra_plugin_sdk::prelude::*;

use game::Game;
use settings::Settings;
use state::AcceptState;

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
        triggers::definitions()
    }

    #[hook]
    async fn on_config(&self, _ctx: &PluginContext, config: Settings) {
        self.state.apply_settings(&config);
        self.settings.store(config);
    }

    #[hook]
    async fn on_start(&self, ctx: &PluginContext) -> anyhow::Result<()> {
        self.state.apply_settings(&self.settings.load());
        #[cfg(windows)]
        tokio::spawn(
            watcher::Watcher::new(ctx.clone(), self.settings.clone(), self.state.clone()).run(),
        );
        #[cfg(not(windows))]
        let _ = ctx;
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
