#![feature(iter_intersperse)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gold_pass_bot::{
    ClanTag, ExcelStats, FileStorage, RaidMember, RaidWeekendStats, Replicated, S3Storage, Season,
    Storage, StorageBackend,
};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::model::prelude::AttachmentType;
use serenity::prelude::*;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

use arc_swap::ArcSwap;

#[group]
#[commands(stats, export)]
struct General;

struct Handler;

struct ClanStates;
impl TypeMapKey for ClanStates {
    type Value = Arc<ArcSwap<(Storage, u64)>>;
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let layers = tracing_subscriber::registry()
        .with(gold_pass_bot::TracingCrateFilter {})
        .with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(layers).unwrap();

    let store_path = std::env::var("STORE_PATH").unwrap_or_else(|_| "data.json".to_string());
    let api_path = std::env::var("API_PATH").unwrap_or_else(|_| "api.key".to_string());

    let s3_bucket = std::env::var("S3_BUCKET").unwrap();
    let s3_access_key = std::env::var("S3_ACCESS_KEY").unwrap();
    let s3_secret_key = std::env::var("S3_SECRET_KEY").unwrap();
    let s3_endpoint = std::env::var("S3_ENDPOINT").unwrap();

    #[cfg(not(debug_assertions))]
    let prefix = "!";
    #[cfg(debug_assertions)]
    let prefix = "+";
    let framework = StandardFramework::new()
        .configure(|c| c.prefix(prefix))
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN")
        .expect("Discord Token should be set using the `DISCORD_TOKEN` environment variable");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let mut storage_backend: Box<dyn StorageBackend> = Box::new(Replicated::new(
        FileStorage::new(store_path.clone()),
        S3Storage::new(
            s3::Bucket::new(
                &s3_bucket,
                s3::Region::Custom {
                    region: "default".to_string(),
                    endpoint: s3_endpoint.to_string(),
                },
                s3::creds::Credentials::new(
                    Some(&s3_access_key),
                    Some(&s3_secret_key),
                    None,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap()
            .with_path_style(),
        ),
    ));

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let storage = Storage::load(storage_backend.as_mut())
        .await
        .unwrap_or_else(|_| Storage::empty());
    let shared_storage = Arc::new(ArcSwap::new(Arc::new((storage.clone(), elapsed))));
    {
        let mut data = client.data.write().await;
        data.insert::<ClanStates>(shared_storage.clone());
    }

    tokio::spawn(async move {
        let raw_key = tokio::fs::read_to_string(api_path).await.unwrap();
        let key = raw_key
            .as_str()
            .strip_suffix("\n")
            .unwrap_or(raw_key.as_str());
        let client = gold_pass_bot::Client::new(key.to_string());

        let alfie_tag = "#2L99VLJ9P";

        let mut storage = storage;
        storage.register_clan(ClanTag(alfie_tag.to_string()));

        loop {
            let season = Season::current();

            for tag in [ClanTag(alfie_tag.to_string())] {
                let update_span = tracing::span!(tracing::Level::INFO, "UpdateClanStats");
                let _tmp = update_span.enter();

                tracing::debug!("Updating Clan Stats: {:?}", tag);

                let clan_season_stats = match storage.get_mut(&tag, &season) {
                    Some(s) => s,
                    None => {
                        tracing::error!(
                            "Getting Stats entry for Clan {:?} and Season {:?}",
                            tag,
                            season
                        );
                        continue;
                    }
                };

                gold_pass_bot::update_names(&client, &tag, clan_season_stats).await;

                gold_pass_bot::update_war(&client, &tag, &mut storage).await;

                gold_pass_bot::update_cwl(&client, &tag, &mut storage).await;

                gold_pass_bot::update_clan_games(&client, &tag, &mut storage).await;

                match client.captial_raid_seasons(&tag).await {
                    Ok(raid_res) => {
                        for raid in raid_res.items {
                            tracing::debug!("Start-Time: {:?}", raid.startTime);
                            let members = match raid.members {
                                Some(m) => m
                                    .into_iter()
                                    .map(|member| {
                                        (
                                            member.tag,
                                            RaidMember {
                                                looted: member.capitalResourcesLooted,
                                            },
                                        )
                                    })
                                    .collect(),
                                None => {
                                    tracing::trace!(
                                        "Skipping weekend, because there is no member list"
                                    );

                                    continue;
                                }
                            };

                            let start_time = raid.startTime;

                            let clan_season_stats =
                                storage.get_mut(&tag, &raid.startTime.into()).unwrap();

                            clan_season_stats.raid_weekend.insert(
                                start_time.clone(),
                                RaidWeekendStats {
                                    start_time,
                                    members,
                                },
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error loading Capital Raid Seasons: {:?}", e);
                    }
                };

                drop(_tmp);
            }

            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            shared_storage.swap(Arc::new((storage.clone(), elapsed)));

            if let Err(e) = storage.save(storage_backend.as_mut()).await {
                tracing::error!("Saving Storage: {:?}", e);
            }

            tracing::info!("Done Updating Stats");

            tokio::time::sleep(Duration::from_secs(90)).await;
        }
    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn stats(ctx: &Context, msg: &Message) -> CommandResult {
    let guard = ctx.data.read().await;
    let storage: &Arc<ArcSwap<_>> = guard.get::<ClanStates>().unwrap();

    let stats_guard = storage.load();
    let (stats, timestamp) = stats_guard.as_ref();

    let alfie_tag = ClanTag("#2L99VLJ9P".to_string());

    let season = Season::current();
    tracing::trace!("Displaying stats for season: {:?}", season);

    let clan_stats = stats.get(&alfie_tag, &season).expect("");

    let player_count = clan_stats.players_summary().count();
    tracing::trace!("Sending Summary for {} Players", player_count);

    let max_name_length = clan_stats
        .players_summary()
        .map(|(n, _)| {
            clan_stats
                .player_names
                .get(&n)
                .map(|n| n.len())
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);

    let padding_width = 11.max(max_name_length);
    let header_padding_width = padding_width.saturating_sub(11);

    let player_summaries: BTreeMap<_, _> = clan_stats
        .players_summary()
        .map(|(tag, v)| (clan_stats.player_names.get(&tag).unwrap(), v))
        .collect();

    const PLAYER_PER_MESSAGE: usize = 35;
    for batch in 0..(player_count / PLAYER_PER_MESSAGE + 1) {
        tracing::trace!("Sending Batch: {}", batch);

        let summary: String = core::iter::once(format!(
            "```Player Tag {:width$}| CWL | Wars | Raids | Games",
            ' ',
            width = header_padding_width
        ))
        .chain(core::iter::once(
            core::iter::repeat('-')
                .take(40 + padding_width.saturating_sub(11))
                .collect(),
        ))
        .chain(
            player_summaries
                .iter()
                .skip(batch * PLAYER_PER_MESSAGE)
                .take(PLAYER_PER_MESSAGE)
                .map(|(name, sum)| {
                    format!(
                        "{:width$}|  {:2} |   {:2} | {:5} |    {:2}",
                        name,
                        sum.cwl_stars,
                        sum.war_stars,
                        sum.raid_loot,
                        sum.games_score,
                        width = padding_width
                    )
                }),
        )
        .chain(core::iter::once("```".to_string()))
        .intersperse("\n".to_string())
        .collect();
        tracing::trace!("Summary message length: {:?}", summary.len());

        if let Err(e) = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.content(format!(
                    "{}\nTimestamp: {}\n{}/{}",
                    summary,
                    timestamp,
                    batch + 1,
                    player_count / 25 + 1
                ))
            })
            .await
        {
            tracing::error!("Sending Response: {:?}", e);
        }
    }

    Ok(())
}

#[command]
async fn export(ctx: &Context, msg: &Message) -> CommandResult {
    let guard = ctx.data.read().await;
    let storage: &Arc<ArcSwap<_>> = guard.get::<ClanStates>().unwrap();

    let stats_guard = storage.load();
    let (stats, _timestamp) = stats_guard.as_ref(); // TODO

    let alfie_tag = ClanTag("#2L99VLJ9P".to_string());

    let current_season = Season::current();
    let last_season = current_season.previous();

    tracing::trace!(
        "Displaying stats for seasons: {:?}, {:?}",
        current_season,
        last_season
    );

    let files = [current_season, last_season].map(|season| {
        let clan_stats = stats.get(&alfie_tag, &season).expect("");

        let mut excel_book = ExcelStats::new().populate_workbook(clan_stats);
        let content = excel_book.save_to_buffer().unwrap();

        AttachmentType::Bytes {
            data: Cow::Owned(content),
            filename: format!("Tracker - {}-{}.xlsx", season.month, season.year),
        }
    });

    if let Err(e) = msg
        .channel_id
        .send_files(&ctx.http, files, |m| m.content("Populated Spreadsheet"))
        .await
    {
        tracing::error!("Sending Excel Stats: {:?}", e);
    }

    Ok(())
}
