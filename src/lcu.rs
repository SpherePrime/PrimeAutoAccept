use std::path::{Path, PathBuf};
use std::time::Duration;

use astra_plugin_sdk::prelude::*;

use crate::game::Game;
use crate::process;

const DEFAULT_INSTALL_DIRS: [&str; 2] = [
    r"C:\Riot Games\League of Legends",
    r"D:\Riot Games\League of Legends",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lockfile {
    pub port: u16,
    pub password: String,
}

impl Lockfile {
    pub fn parse(contents: &str) -> Option<Lockfile> {
        let parts: Vec<&str> = contents.trim().split(':').collect();
        if parts.len() < 5 {
            return None;
        }
        Some(Lockfile {
            port: parts[2].parse().ok()?,
            password: parts[3].to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadyCheck {
    None,
    Pending,
    Answered,
}

pub struct LcuClient {
    http: reqwest::Client,
    lockfile: Lockfile,
}

impl LcuClient {
    pub fn connect(custom_dir: &str) -> Option<LcuClient> {
        let lockfile = locate_lockfile(custom_dir)?;
        let contents = std::fs::read_to_string(lockfile).ok()?;
        let lockfile = Lockfile::parse(&contents)?;
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(3))
            .build()
            .ok()?;
        Some(LcuClient { http, lockfile })
    }

    fn url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{}", self.lockfile.port, path)
    }

    pub async fn ready_check(&self) -> anyhow::Result<ReadyCheck> {
        let response = self
            .http
            .get(self.url("/lol-matchmaking/v1/ready-check"))
            .basic_auth("riot", Some(&self.lockfile.password))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(ReadyCheck::None);
        }
        let body: serde_json::Value = serde_json::from_str(&response.text().await?)?;
        let state = body["state"].as_str().unwrap_or_default();
        let answer = body["playerResponse"].as_str().unwrap_or_default();
        Ok(match (state, answer) {
            ("InProgress", "None") => ReadyCheck::Pending,
            ("InProgress", _) => ReadyCheck::Answered,
            _ => ReadyCheck::None,
        })
    }

    pub async fn accept(&self) -> anyhow::Result<()> {
        let response = self
            .http
            .post(self.url("/lol-matchmaking/v1/ready-check/accept"))
            .basic_auth("riot", Some(&self.lockfile.password))
            .send()
            .await?;
        anyhow::ensure!(
            response.status().is_success(),
            "LCU answered {}",
            response.status()
        );
        Ok(())
    }
}

fn locate_lockfile(custom_dir: &str) -> Option<PathBuf> {
    let from_process = process::find_pid(Game::LeagueOfLegends.process_name())
        .and_then(process::executable_path)
        .and_then(|exe| exe.parent().map(|dir| dir.join("lockfile")));
    let custom = (!custom_dir.trim().is_empty()).then(|| Path::new(custom_dir.trim()).join("lockfile"));
    from_process
        .into_iter()
        .chain(custom)
        .chain(DEFAULT_INSTALL_DIRS.iter().map(|dir| Path::new(dir).join("lockfile")))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lockfile() {
        let lockfile = Lockfile::parse("LeagueClient:1234:54321:secret:https").unwrap();
        assert_eq!(lockfile.port, 54321);
        assert_eq!(lockfile.password, "secret");
        assert!(Lockfile::parse("broken").is_none());
    }
}
