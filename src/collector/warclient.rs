use reqwest::StatusCode;
use serde::Deserialize;

use crate::{ClanBadges, ClanTag, Client, LoadError, PlayerGamesStats, PlayerTag, Time, WarLog};

pub struct WarClient<'c> {
    client: &'c Client,
}

impl<'c> WarClient<'c> {
    pub fn from_client(client: &'c Client) -> Self {
        Self { client }
    }

    pub async fn logs(&self, clan: &ClanTag) -> Result<WarLog, LoadError> {
        let resp = self
            .client
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/warlog?limit=10",
                clan.0.as_str().strip_prefix("#").unwrap_or(clan.0.as_str())
            ))
            .bearer_auth(&self.client.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                return Err(LoadError::ReqwestError(e));
            }
        };

        if resp.status() != StatusCode::OK {
            return Err(LoadError::NotOkResponse(resp.status()));
        }

        resp.json().await.map_err(|e| LoadError::Deserialize(e))
    }

    pub async fn current(&self, clan: &ClanTag) -> Result<CurrentWar, LoadError> {
        let resp = self
            .client
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/currentwar",
                clan.0.as_str().strip_prefix("#").unwrap_or(clan.0.as_str())
            ))
            .bearer_auth(&self.client.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("{:?}", e);
                return Err(LoadError::ReqwestError(e));
            }
        };

        if resp.status() != StatusCode::OK {
            return Err(LoadError::NotOkResponse(resp.status()));
        }

        resp.json().await.map_err(|e| LoadError::Deserialize(e))
    }
}

#[derive(Debug, Deserialize)]
pub struct CurrentWar {
    pub state: CurrentWarState,
    pub clan: WarClan,
    opponent: WarClan,
    #[serde(rename = "teamSize")]
    team_size: Option<usize>,
    #[serde(rename = "attacksPerMember")]
    attacks_per_member: Option<usize>,
    #[serde(rename = "startTime")]
    pub start_time: Option<Time>,
    #[serde(rename = "endTime")]
    end_time: Option<String>,
    #[serde(rename = "preparationStartTime")]
    preparation_start_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum CurrentWarState {
    #[serde(rename = "clanNotFound")]
    ClanNotFound,
    #[serde(rename = "accessDenied")]
    AccessDenied,
    #[serde(rename = "notInWar")]
    NotInWar,
    #[serde(rename = "inMatchmaking")]
    InMatchmaking,
    #[serde(rename = "enterWar")]
    EnterWar,
    #[serde(rename = "matched")]
    Matched,
    #[serde(rename = "preparation")]
    Preparation,
    #[serde(rename = "war")]
    War,
    #[serde(rename = "inWar")]
    InWar,
    #[serde(rename = "warEnded")]
    Ended,
}

#[derive(Debug, Deserialize)]
pub struct WarClan {
    tag: Option<ClanTag>,
    name: Option<String>,
    attacks: usize,
    #[serde(rename = "badgeUrls")]
    badge_urls: ClanBadges,
    #[serde(rename = "clanLevel")]
    clan_level: usize,
    #[serde(rename = "destructionPercentage")]
    destruction_percentage: f32,
    stars: usize,
    #[serde(rename = "expEarned")]
    exp_earned: Option<f32>,
    pub members: Option<Vec<WarClanMember>>,
}

#[derive(Debug, Deserialize)]
pub struct WarClanMember {
    mapPosition: usize,
    name: String,
    opponentAttacks: usize,
    pub tag: PlayerTag,
    townhallLevel: usize,
    #[serde(default)]
    pub attacks: Vec<WarClanMemberAttack>,
    bestOpponentAttack: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct WarClanMemberAttack {
    pub attackerTag: PlayerTag,
    pub defenderTag: PlayerTag,
    pub destructionPercentage: usize,
    pub duration: usize,
    pub order: usize,
    pub stars: usize,
}
