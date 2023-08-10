use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::{ClanTag, ClanWarLeagueSeason, PlayerTag, Time};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Storage {
    clans: HashMap<ClanTag, HashMap<Season, ClanStorage>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Season {
    pub year: usize,
    pub month: usize,
}

impl<'de> Deserialize<'de> for Season {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;

        let (raw_year, raw_month) = raw.split_once('-').ok_or(serde::de::Error::custom(""))?;

        let year = raw_year.parse().map_err(|e| serde::de::Error::custom(e))?;
        let month = raw_month.parse().map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self { year, month })
    }
}
impl Serialize for Season {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = format!("{:04}-{:02}", self.year, self.month);
        raw.serialize(serializer)
    }
}

impl From<Time> for Season {
    fn from(value: crate::Time) -> Self {
        Self {
            year: value.year,
            month: value.month,
        }
    }
}
impl From<ClanWarLeagueSeason> for Season {
    fn from(value: ClanWarLeagueSeason) -> Self {
        Self {
            year: value.year,
            month: value.month,
        }
    }
}

impl Season {
    pub fn current() -> Self {
        let now = chrono::Utc::now();
        Self {
            year: now.year() as usize,
            month: now.month() as usize,
        }
    }
}

/// All the Stats for a single Clan
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ClanStorage {
    /// All the CWL related Stats
    pub cwl: CwlStats,
    /// All the War related Stats
    pub wars: HashMap<Time, WarStats>,
    pub games: HashMap<PlayerTag, PlayerGamesStats>,
    pub raid_weekend: HashMap<Time, RaidWeekendStats>,
    pub player_names: HashMap<PlayerTag, String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PlayerGamesStats {
    score: usize,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CwlStats {
    pub wars: Vec<CwlWarStats>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CwlWarStats {
    pub members: HashMap<PlayerTag, MemberWarStats>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WarStats {
    pub start_time: Time,
    pub members: HashMap<PlayerTag, MemberWarStats>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemberWarStats {
    pub attacks: Vec<WarAttack>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WarAttack {
    pub destruction: usize,
    pub stars: usize,
    pub duration: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidWeekendStats {
    pub start_time: Time,
    pub members: HashMap<PlayerTag, RaidMember>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidMember {
    pub looted: usize,
}

impl Storage {
    pub fn empty() -> Self {
        Self {
            clans: HashMap::new(),
        }
    }

    pub fn register_clan(&mut self, tag: ClanTag) {
        if self.clans.contains_key(&tag) {
            return;
        }

        self.clans.insert(tag, HashMap::new());
    }

    pub fn get_mut(&mut self, tag: &ClanTag, season: &Season) -> Option<&mut ClanStorage> {
        self.clans.get_mut(tag).map(|seasons| {
            if !seasons.contains_key(season) {
                seasons.insert(season.clone(), ClanStorage::default());
            }

            seasons.get_mut(season).unwrap()
        })
    }

    pub fn get(&self, tag: &ClanTag, season: &Season) -> Option<&ClanStorage> {
        self.clans.get(tag).and_then(|s| s.get(season))
    }

    pub async fn load<P>(path: P) -> Result<Self, ()>
    where
        P: AsRef<Path>,
    {
        let content = tokio::fs::read(path).await.map_err(|e| ())?;
        serde_json::from_slice(&content).map_err(|e| ())
    }

    pub async fn save<P>(&self, path: P) -> Result<(), ()>
    where
        P: AsRef<Path>,
    {
        let content = serde_json::to_vec(&self).map_err(|e| {
            tracing::error!("Serializing {:?}", e);
            ()
        })?;

        let path = path.as_ref();
        if !path.exists() {
            tokio::fs::File::create(path).await.map_err(|e| {
                tracing::error!("Creating file {:?}", e);
                ()
            })?;
        }
        tokio::fs::write(path, content).await.map_err(|e| {
            tracing::error!("Writing file {:?}", e);
            ()
        })?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct PlayerSummary {
    pub cwl_stars: usize,
    pub war_stars: usize,
    pub raid_loot: usize,
    pub games_score: usize,
}

impl ClanStorage {
    pub fn players_summary(&self) -> impl Iterator<Item = (PlayerTag, PlayerSummary)> + '_ {
        // TODO
        // Get all the players we have some data for
        let players: HashSet<PlayerTag> = self.player_names.keys().cloned().collect();

        players.into_iter().map(|ptag| {
            let cwl_stars: usize = self
                .cwl
                .wars
                .iter()
                .map(|war| {
                    war.members
                        .get(&ptag)
                        .map(|mstats| mstats.attacks.iter().map(|a| a.stars).sum::<usize>())
                        .unwrap_or(0)
                })
                .sum();

            let war_stars: usize = self
                .wars
                .values()
                .map(|war| {
                    war.members
                        .get(&ptag)
                        .map(|mstats| mstats.attacks.iter().map(|att| att.stars).sum::<usize>())
                        .unwrap_or(0)
                })
                .sum();

            let raid_loot: usize = self
                .raid_weekend
                .values()
                .map(|raid| {
                    raid.members
                        .get(&ptag)
                        .map(|rstats| rstats.looted)
                        .unwrap_or(0)
                })
                .sum();

            let games_score = self.games.get(&ptag).map(|s| s.score).unwrap_or(0);

            (
                ptag,
                PlayerSummary {
                    cwl_stars,
                    war_stars,
                    raid_loot,
                    games_score,
                },
            )
        })
    }
}
