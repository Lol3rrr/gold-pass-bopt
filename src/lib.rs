use std::borrow::Cow;

use reqwest::StatusCode;
use serde::{de::Visitor, Deserialize};
use serenity::futures::TryFutureExt;

mod clans;
pub use clans::*;

pub struct Client {
    client: reqwest::Client,
    api_key: Cow<'static, str>,
}

#[derive(Debug, Deserialize)]
pub struct WarLog {
    items: Vec<WarLogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WarLogEntry {
    #[serde(rename = "attacksPerMember")]
    attacks_per_member: usize,
    clan: serde_json::Value,
    #[serde(rename = "endTime")]
    end_time: String,
    opponent: serde_json::Value,
    result: String,
    #[serde(rename = "teamSize")]
    team_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct CurrentWar {
    state: String,
    clan: WarClan,
    opponent: WarClan,
    #[serde(rename = "teamSize")]
    team_size: Option<usize>,
    #[serde(rename = "attacksPerMember")]
    attacks_per_member: Option<usize>,
    #[serde(rename = "startTime")]
    start_time: Option<String>,
    #[serde(rename = "endTime")]
    end_time: Option<String>,
    #[serde(rename = "preparationStartTime")]
    preparation_start_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WarClan {
    tag: Option<ClanTag>,
    name: Option<String>,
    attacks: usize,
    #[serde(rename = "badgeUrls")]
    badge_urls: serde_json::Value,
    #[serde(rename = "clanLevel")]
    clan_level: usize,
    #[serde(rename = "destructionPercentage")]
    destruction_percentage: f32,
    stars: usize,
    #[serde(rename = "expEarned")]
    exp_earned: Option<f32>,
    members: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueGroup {
    tag: Option<String>,
    state: Option<String>,
    season: Option<String>,
    clans: Vec<ClanWarLeagueClan>,
    pub rounds: Vec<ClanWarLeagueRound>,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueRound {
    #[serde(rename = "warTags")]
    pub war_tags: Vec<WarTag>,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueClan {
    tag: ClanTag,
    #[serde(rename = "clanLevel")]
    clan_level: usize,
    name: String,
    members: Vec<ClanWarLeagueClanMember>,
    #[serde(rename = "badgeUrls")]
    badge_urls: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueClanMember {
    tag: PlayerTag,
    #[serde(rename = "townHallLevel")]
    town_hall_level: usize,
    name: String,
}

impl Client {
    pub fn new<S>(api: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self {
            client: reqwest::Client::new(),
            api_key: api.into(),
        }
    }

    pub async fn load_warlog(&self, clan_tag: &str) -> Result<WarLog, ()> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/warlog?limit=10",
                clan_tag.strip_prefix("#").unwrap_or(clan_tag)
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}", e);
                return Err(());
            }
        };

        if resp.status() != StatusCode::OK {
            dbg!(&resp);
            return Err(());
        }

        resp.json().await.map_err(|e| ())
    }

    pub async fn current_war(&self, clan_tag: &str) -> Result<CurrentWar, ()> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/currentwar",
                clan_tag.strip_prefix("#").unwrap_or(clan_tag)
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}", e);
                return Err(());
            }
        };

        resp.json().await.map_err(|e| ())
    }

    pub async fn clan_war_league_group(&self, clan_tag: &str) -> Result<ClanWarLeagueGroup, ()> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/currentwar/leaguegroup",
                clan_tag.strip_prefix("#").unwrap_or(clan_tag)
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}", e);
                return Err(());
            }
        };

        resp.json().await.map_err(|e| panic!("{:?}", e))
    }

    pub async fn clan_war_league_war(&self, war_tag: &WarTag) -> Result<serde_json::Value, ()> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clanwarleagues/wars/%23{}",
                war_tag
                    .0
                    .as_str()
                    .strip_prefix("#")
                    .unwrap_or(war_tag.0.as_str())
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}", e);
                return Err(());
            }
        };

        resp.json().await.map_err(|e| panic!("{:?}", e))
    }
}
