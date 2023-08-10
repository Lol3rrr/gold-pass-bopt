use std::{borrow::Cow, collections::HashMap};

use serde::Deserialize;
use serenity::model::prelude::Member;

use crate::{
    ClanStorage, ClanTag, CwlWarStats, MemberWarStats, PlayerTag, Season, Storage, WarAttack,
    WarStats, WarTag,
};

mod api;

mod warclient;
pub use warclient::*;

mod time;
pub use time::Time;

mod cwl;
pub use cwl::*;

#[derive(Debug)]
pub enum LoadError {
    ReqwestError(reqwest::Error),
    NotOkResponse(reqwest::StatusCode),
    Deserialize(reqwest::Error),
}

/// A Client for the Clash of Clans API
pub struct Client {
    client: reqwest::Client,
    api_key: Cow<'static, str>,
}

#[derive(Debug, Deserialize)]
pub struct ClanBadges {
    pub large: String,
    pub medium: String,
    pub small: String,
}

#[derive(Debug, Deserialize)]
pub struct WarLog {
    pub items: Vec<WarLogEntry>,
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
pub struct ClanInfo {
    warLeague: serde_json::Value,
    capitalLeague: serde_json::Value,
    memberList: Vec<ClanMember>,
    tag: ClanTag,
    requiredVersusTrophies: usize,
    isWarLogPublic: bool,
    clanLevel: usize,
    warWinStreak: usize,
    warWins: usize,
    warTies: usize,
    warLosses: usize,
    clanPoints: usize,
    requiredTownhallLevel: usize,
    chatLanguage: serde_json::Value,
    isFamilyFriendly: bool,
    warFrequency: serde_json::Value,
    clanBuilderBasePoints: usize,
    clanVersusPoints: usize,
    clanCapitalPoints: usize,
    requiredTrophies: usize,
    requiredBuilderBaseTrophies: usize,
    labels: serde_json::Value,
    name: String,
    location: serde_json::Value,
    #[serde(rename = "type")]
    ty: serde_json::Value,
    members: usize,
    description: String,
    clanCapital: serde_json::Value,
    badgeUrls: ClanBadges,
}

#[derive(Debug, Deserialize)]
pub struct ClanMember {
    league: serde_json::Value,
    builderBaseLeague: serde_json::Value,
    versusTrophies: usize,
    tag: PlayerTag,
    name: String,
    role: serde_json::Value,
    expLevel: usize,
    clanRank: usize,
    previousClanRank: usize,
    donations: usize,
    donationsReceived: usize,
    trophies: usize,
    builderBaseTrophies: usize,
    playerHouse: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CapitalRaidWeekend {
    state: String,
    pub startTime: Time,
    endTime: String,
    capitalTotalLoot: usize,
    raidsCompleted: usize,
    totalAttacks: usize,
    enemyDistrictsDestroyed: usize,
    offensiveReward: usize,
    defensiveReward: usize,
    pub members: Option<Vec<CapitalRaidWeekendMember>>,
    pub attackLog: Vec<serde_json::Value>,
    defenseLog: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CapitalRaidWeekendMember {
    attackLimit: usize,
    attacks: usize,
    bonusAttackLimit: usize,
    pub capitalResourcesLooted: usize,
    name: String,
    pub tag: PlayerTag,
}

#[derive(Debug, Deserialize)]
pub struct CapitalRaidWeekendLogs {
    pub items: Vec<CapitalRaidWeekend>,
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

    pub fn war(&self) -> WarClient<'_> {
        WarClient::from_client(self)
    }

    pub async fn clan_war_league_group(
        &self,
        clan_tag: &ClanTag,
    ) -> Result<ClanWarLeagueGroup, LoadError> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/currentwar/leaguegroup",
                clan_tag
                    .0
                    .as_str()
                    .strip_prefix("#")
                    .unwrap_or(clan_tag.0.as_str())
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                return Err(LoadError::ReqwestError(e));
            }
        };

        resp.json().await.map_err(|e| LoadError::Deserialize(e))
    }

    pub async fn clan_war_league_war(
        &self,
        war_tag: &WarTag,
    ) -> Result<ClanWarLeagueWar, LoadError> {
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
                return Err(LoadError::ReqwestError(e));
            }
        };

        resp.json().await.map_err(|e| panic!("{:?}", e))
    }

    pub async fn clan_info(&self, clan: &ClanTag) -> Result<ClanInfo, LoadError> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}",
                clan.0.as_str().strip_prefix("#").unwrap_or(clan.0.as_str())
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                return Err(LoadError::ReqwestError(e));
            }
        };

        resp.json().await.map_err(|e| LoadError::Deserialize(e))
    }

    pub async fn captial_raid_seasons(
        &self,
        clan: &ClanTag,
    ) -> Result<CapitalRaidWeekendLogs, LoadError> {
        let resp = self
            .client
            .get(format!(
                "https://api.clashofclans.com/v1/clans/%23{}/capitalraidseasons",
                clan.0.as_str().strip_prefix("#").unwrap_or(clan.0.as_str())
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                return Err(LoadError::ReqwestError(e));
            }
        };

        resp.json().await.map_err(|e| LoadError::Deserialize(e))
    }
}

#[tracing::instrument(skip(client, clan_season_stats))]
pub async fn update_names(client: &Client, clan: &ClanTag, clan_season_stats: &mut ClanStorage) {
    let info = client.clan_info(clan).await.unwrap();

    clan_season_stats.player_names.clear();
    for member in info.memberList {
        clan_season_stats
            .player_names
            .insert(member.tag, member.name);
    }
}

#[tracing::instrument(skip(client, storage))]
pub async fn update_cwl(client: &Client, clan: &ClanTag, storage: &mut Storage) {
    let w = match client.clan_war_league_group(clan).await {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Loading Clan War League Group: {:?}", e);
            return;
        }
    };

    let war_season: Season = w.season.into();
    let clan_season_stats = storage.get_mut(clan, &war_season).unwrap();

    for (round_index, round) in w.rounds.iter().enumerate() {
        for wtag in round.war_tags.iter() {
            if wtag.0.as_str() == "#0" {
                continue;
            }

            // tracing::debug!("War-Tag: {:?}", tag);
            if let Ok(w) = client.clan_war_league_war(&wtag).await {
                if &w.clan.tag != clan && &w.opponent.tag != clan {
                    continue;
                }

                let clan = if &w.clan.tag == clan {
                    w.clan
                } else {
                    w.opponent
                };

                if clan_season_stats.cwl.wars.len() <= round_index {
                    clan_season_stats.cwl.wars.extend(
                        (0..(clan_season_stats.cwl.wars.len() - round_index) + 1).map(|_| {
                            CwlWarStats {
                                members: HashMap::new(),
                            }
                        }),
                    );
                }
                let cwl_stats = clan_season_stats.cwl.wars.get_mut(round_index).expect("");

                for member in clan.members.iter() {
                    let member_stats =
                        cwl_stats
                            .members
                            .entry(member.tag.clone())
                            .or_insert_with(|| MemberWarStats {
                                attacks: Vec::new(),
                            });

                    if let Some(attacks) = &member.attacks {
                        member_stats.attacks = attacks
                            .iter()
                            .map(|raw_attack| WarAttack {
                                destruction: raw_attack.destructionPercentage,
                                stars: raw_attack.stars,
                                duration: raw_attack.duration,
                            })
                            .collect();
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(client, storage))]
pub async fn update_war(client: &Client, clan_tag: &ClanTag, storage: &mut Storage) {
    let war = match client.war().current(&clan_tag).await {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Error loading War: {:?}", e);
            return;
        }
    };

    if !matches!(war.state, CurrentWarState::InWar) {
        tracing::info!("WAR: Not in War currently {:?}", war.state);
        return;
    }

    let clan = war.clan;
    let members = match clan.members {
        Some(m) => m,
        None => {
            tracing::error!("Current War Clan is missing Members");
            return;
        }
    };

    let start_time = match war.start_time {
        Some(t) => t,
        None => {
            tracing::error!("Current War missing Start Time");
            return;
        }
    };

    let season: Season = start_time.clone().into();

    let clan_season_stats = storage.get_mut(clan_tag, &season).unwrap();

    // clan_season_stats.wars.insert(start_time, WarStats {});

    let war_stats = WarStats {
        start_time: start_time.clone(),
        members: members
            .into_iter()
            .filter_map(|member| {
                let war_attacks = member
                    .attacks
                    .into_iter()
                    .map(|rattack| WarAttack {
                        destruction: rattack.destructionPercentage,
                        stars: rattack.stars,
                        duration: rattack.duration,
                    })
                    .collect();

                Some((
                    member.tag,
                    MemberWarStats {
                        attacks: war_attacks,
                    },
                ))
            })
            .collect(),
    };
    clan_season_stats.wars.insert(start_time, war_stats);
}
