use std::sync::Arc;
use std::time::{Duration, Instant};

use astra_plugin_sdk::prelude::*;
use tokio::task::spawn_blocking;

use crate::detector::{self, Point};
use crate::game::Game;
use crate::lcu::{LcuClient, ReadyCheck};
use crate::process;
use crate::settings::Settings;
use crate::state::AcceptState;
use crate::window::{self, GameWindow};

pub const MATCH_FOUND: &str = "match_found";
pub const MATCH_ACCEPTED: &str = "match_accepted";

const TICK: Duration = Duration::from_millis(500);
const SCREEN_CONFIRM: Duration = Duration::from_millis(600);
const SCREEN_COOLDOWN: Duration = Duration::from_secs(10);

pub struct Watcher {
    ctx: PluginContext,
    settings: Arc<Config<Settings>>,
    state: Arc<AcceptState>,
    lol: LolWatch,
    dota2: ScreenWatch,
    cs2: ScreenWatch,
}

#[derive(Default)]
struct LolWatch {
    client: Option<LcuClient>,
    pending_since: Option<Instant>,
}

#[derive(Default)]
struct ScreenWatch {
    pending_since: Option<Instant>,
    cooldown_until: Option<Instant>,
}

struct ButtonHit {
    window: GameWindow,
    target: Point,
}

impl Watcher {
    pub fn new(ctx: PluginContext, settings: Arc<Config<Settings>>, state: Arc<AcceptState>) -> Self {
        Watcher {
            ctx,
            settings,
            state,
            lol: LolWatch::default(),
            dota2: ScreenWatch::default(),
            cs2: ScreenWatch::default(),
        }
    }

    pub async fn run(mut self) {
        spawn_blocking(window::enable_dpi_awareness).await.ok();
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            for game in Game::ALL {
                let pid = process::find_pid(game.process_name());
                self.state.set_running(game, pid.is_some());
                let active = self.state.is_enabled(game);
                match (game, pid, active) {
                    (Game::LeagueOfLegends, Some(_), true) => self.tick_lol().await,
                    (Game::LeagueOfLegends, _, _) => self.lol = LolWatch::default(),
                    (_, Some(pid), true) => self.tick_screen(game, pid).await,
                    (_, _, _) => *self.screen_watch(game) = ScreenWatch::default(),
                }
            }
        }
    }

    fn delay(&self) -> Duration {
        Duration::from_millis(self.settings.load().accept_delay_ms.min(10_000))
    }

    fn screen_watch(&mut self, game: Game) -> &mut ScreenWatch {
        match game {
            Game::Cs2 => &mut self.cs2,
            _ => &mut self.dota2,
        }
    }

    async fn tick_lol(&mut self) {
        if self.lol.client.is_none() {
            let install_dir = self.settings.load().lol_install_dir.clone();
            self.lol.client = LcuClient::connect(&install_dir);
        }
        let Some(client) = &self.lol.client else {
            return;
        };
        match client.ready_check().await {
            Ok(ReadyCheck::Pending) => {
                let since = match self.lol.pending_since {
                    Some(since) => since,
                    None => {
                        let now = Instant::now();
                        self.lol.pending_since = Some(now);
                        self.announce(MATCH_FOUND, Game::LeagueOfLegends).await;
                        now
                    }
                };
                if since.elapsed() >= self.delay() {
                    match client.accept().await {
                        Ok(()) => self.accepted(Game::LeagueOfLegends).await,
                        Err(error) => self.warn(Game::LeagueOfLegends, &error.to_string()).await,
                    }
                }
            }
            Ok(_) => self.lol.pending_since = None,
            Err(_) => self.lol = LolWatch::default(),
        }
    }

    async fn tick_screen(&mut self, game: Game, pid: u32) {
        let delay = self.delay().max(SCREEN_CONFIRM);
        let focus_game = self.settings.load().focus_game;
        let watch = self.screen_watch(game);
        if watch.cooldown_until.is_some_and(|until| Instant::now() < until) {
            return;
        }
        let pending_since = watch.pending_since;
        let Some(hit) = spawn_blocking(move || locate_button(pid)).await.ok().flatten() else {
            self.screen_watch(game).pending_since = None;
            return;
        };
        let Some(since) = pending_since else {
            self.screen_watch(game).pending_since = Some(Instant::now());
            self.announce(MATCH_FOUND, game).await;
            return;
        };
        if since.elapsed() < delay || (!focus_game && !hit.window.is_foreground()) {
            return;
        }
        *self.screen_watch(game) = ScreenWatch {
            pending_since: None,
            cooldown_until: Some(Instant::now() + SCREEN_COOLDOWN),
        };
        spawn_blocking(move || press(hit)).await.ok();
        self.accepted(game).await;
    }

    async fn accepted(&mut self, game: Game) {
        self.state.record_accept(game);
        if game == Game::LeagueOfLegends {
            self.lol.pending_since = None;
        }
        self.announce(MATCH_ACCEPTED, game).await;
    }

    async fn announce(&self, trigger: &str, game: Game) {
        let _ = self
            .ctx
            .host()
            .log_info(&format!("{}: {}", game.title(), trigger))
            .await;
        if self.ctx.active_triggers().contains(trigger) {
            let payload = serde_json::json!({ "game": game.id(), "title": game.title() });
            let _ = self.ctx.host().fire_trigger(trigger, &payload.to_string()).await;
        }
    }

    async fn warn(&self, game: Game, message: &str) {
        let _ = self
            .ctx
            .host()
            .log_warn(&format!("{}: {}", game.title(), message))
            .await;
    }
}

fn locate_button(pid: u32) -> Option<ButtonHit> {
    let window = GameWindow::of_process(pid)?;
    let target = locate_button_in(window)?;
    Some(ButtonHit { window, target })
}

fn press(hit: ButtonHit) {
    hit.window.bring_to_front();
    std::thread::sleep(Duration::from_millis(250));
    let target = locate_button_in(hit.window).unwrap_or(hit.target);
    window::click_at(target);
}

fn locate_button_in(window: GameWindow) -> Option<Point> {
    let capture = window.capture()?;
    let point = detector::find_accept_button(&capture.frame)?;
    Some(Point {
        x: capture.origin.x + point.x,
        y: capture.origin.y + point.y,
    })
}
