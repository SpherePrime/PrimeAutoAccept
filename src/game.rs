#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Game {
    LeagueOfLegends,
    Dota2,
    Cs2,
}

impl Game {
    pub const ALL: [Game; 3] = [Game::LeagueOfLegends, Game::Dota2, Game::Cs2];

    pub fn id(self) -> &'static str {
        match self {
            Game::LeagueOfLegends => "lol",
            Game::Dota2 => "dota2",
            Game::Cs2 => "cs2",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Game::LeagueOfLegends => "League of Legends",
            Game::Dota2 => "Dota 2",
            Game::Cs2 => "Counter-Strike 2",
        }
    }

    pub fn process_name(self) -> &'static str {
        match self {
            Game::LeagueOfLegends => "LeagueClientUx.exe",
            Game::Dota2 => "dota2.exe",
            Game::Cs2 => "cs2.exe",
        }
    }

    pub fn parse(raw: &str) -> Option<Game> {
        let normalized: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        match normalized.as_str() {
            "lol" | "league" | "leagueoflegends" => Some(Game::LeagueOfLegends),
            "dota" | "dota2" => Some(Game::Dota2),
            "cs" | "cs2" | "counterstrike" | "counterstrike2" => Some(Game::Cs2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases() {
        assert_eq!(Game::parse("League of Legends"), Some(Game::LeagueOfLegends));
        assert_eq!(Game::parse("Dota 2"), Some(Game::Dota2));
        assert_eq!(Game::parse("Counter-Strike 2"), Some(Game::Cs2));
        assert_eq!(Game::parse("valorant"), None);
    }
}
