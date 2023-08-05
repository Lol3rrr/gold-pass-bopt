use serde::Deserialize;

use crate::{ClanBadges, ClanTag, PlayerTag, WarTag};

#[derive(Debug)]
pub struct ClanWarLeagueSeason {
    pub year: usize,
    pub month: usize,
}

impl<'de> Deserialize<'de> for ClanWarLeagueSeason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;

        let (raw_year, raw_month) = raw.split_once('-').ok_or(serde::de::Error::custom(""))?;

        let year: usize = raw_year.parse().map_err(serde::de::Error::custom)?;
        let month: usize = raw_month.parse().map_err(serde::de::Error::custom)?;

        Ok(Self { year, month })
    }
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueGroup {
    tag: Option<String>,
    state: Option<String>,
    pub season: ClanWarLeagueSeason,
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
    badge_urls: ClanBadges,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueClanMember {
    tag: PlayerTag,
    #[serde(rename = "townHallLevel")]
    town_hall_level: usize,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueWar {
    pub state: ClanWarLeagueWarState,
    teamSize: usize,
    preparationStartTime: String,
    startTime: String,
    endTime: String,
    pub clan: ClanWarLeagueWarClan,
    pub opponent: ClanWarLeagueWarClan,
    warStartTime: String,
}

#[derive(Debug, Deserialize)]
pub enum ClanWarLeagueWarState {
    groupNotFound,
    notInWar,
    #[serde(rename = "preparation")]
    Preparation,
    #[serde(rename = "inWar")]
    War,
    #[serde(rename = "warEnded")]
    Ended,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueWarClan {
    badgeUrls: ClanBadges,
    attacks: usize,
    clanLevel: usize,
    destructionPercentage: f32,
    pub members: Vec<ClanWarLeagueWarMember>,
    name: String,
    stars: usize,
    pub tag: ClanTag,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueWarMember {
    pub tag: PlayerTag,
    pub name: String,
    pub mapPosition: usize,
    pub townhallLevel: usize,
    pub opponentAttacks: usize,
    pub bestOpponentAttack: Option<ClanWarLeagueAttack>,
    pub attacks: Option<Vec<ClanWarLeagueAttack>>,
}

#[derive(Debug, Deserialize)]
pub struct ClanWarLeagueAttack {
    pub attackerTag: PlayerTag,
    pub defenderTag: PlayerTag,
    pub destructionPercentage: usize,
    pub duration: usize,
    pub order: usize,
    pub stars: usize,
}
