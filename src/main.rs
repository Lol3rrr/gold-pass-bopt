#![feature(iter_intersperse)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gold_pass_bot::{
    ClanTag, ExcelStats, PlayerSummary, RaidMember, RaidWeekendStats, Season, Storage,
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

static REGISTRY: once_cell::sync::Lazy<prometheus::Registry> =
    once_cell::sync::Lazy::new(|| prometheus::Registry::new());

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let layers = tracing_subscriber::registry()
        .with(gold_pass_bot::TracingCrateFilter {})
        .with(tracing_subscriber::fmt::layer().with_ansi(false));
    tracing::subscriber::set_global_default(layers).unwrap();

    let args = clap::Command::new("Gold-Pass-Bot")
        .subcommand(
            clap::Command::new("bot").arg(
                clap::Arg::new("storage")
                    .long("storage")
                    .value_names(["storage-target"]),
            ),
        )
        .get_matches();

    let mut storage_backend = args
        .subcommand_matches("bot")
        .unwrap()
        .get_one::<String>("storage")
        .map(|arg: &String| gold_pass_bot::parse_storage(&arg))
        .unwrap()
        .unwrap();

    let api_path = std::env::var("API_PATH").unwrap_or_else(|_| "api.key".to_string());

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

        let error_counter =
            prometheus::Counter::new("api_errors", "The Number of errors returned by the API")
                .unwrap();

        once_cell::sync::Lazy::force(&REGISTRY)
            .register(Box::new(error_counter.clone()))
            .unwrap();

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

                if let Err(e) = gold_pass_bot::update_names(&client, &tag, clan_season_stats).await
                {
                    error_counter.inc();
                }

                if let Err(e) = gold_pass_bot::update_war(&client, &tag, &mut storage).await {
                    error_counter.inc();
                }

                if let Err(e) = gold_pass_bot::update_cwl(&client, &tag, &mut storage).await {
                    error_counter.inc();
                }

                if let Err(e) = gold_pass_bot::update_clan_games(&client, &tag, &mut storage).await
                {
                    error_counter.inc();
                }

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
                        error_counter.inc();

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

    tokio::spawn(async move {
        let app = axum::Router::new().route("/metrics", axum::routing::get(metrics));

        axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

fn generate_batches(
    player_count: usize,
    header_padding_width: usize,
    padding_width: usize,
    summaries: BTreeMap<&String, PlayerSummary>,
    timestamp: u64,
) -> Vec<String> {
    let mut summary_iter = summaries
        .iter()
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
        })
        .peekable();
    let line_width = format!(
        "{:width$}|  {:2} |   {:2} | {:5} |    {:2}",
        "",
        0,
        0,
        0,
        0,
        width = padding_width
    )
    .len();

    let players_per_line = 1800 / line_width;
    (0..(player_count / players_per_line + 1))
        .map(|batch| {
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
            .chain(summary_iter.by_ref().take(players_per_line))
            .chain(core::iter::once("```".to_string()))
            .intersperse("\n".to_string())
            .collect();

            format!(
                "{}\nTimestamp: {}\n{}/{}",
                summary,
                timestamp,
                batch + 1,
                player_count / players_per_line + 1
            )
        })
        .collect()
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

    if let Err(e) = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.content(format!("Stats for {:02}-{}", season.month, season.year))
        })
        .await
    {
        tracing::error!("Sending Message {:?}", e);
    }

    let batches = generate_batches(
        player_count,
        header_padding_width,
        padding_width,
        player_summaries,
        *timestamp,
    );

    for (batch, content) in batches.into_iter().enumerate() {
        tracing::trace!("Sending Batch: {}", batch);

        tracing::trace!("Summary message length: {:?}", content.len());

        if let Err(e) = msg
            .channel_id
            .send_message(&ctx.http, |m| m.content(content))
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

async fn metrics() -> String {
    let reg = once_cell::sync::Lazy::force(&REGISTRY);

    let encoder = prometheus::TextEncoder::new();
    let metrics_family = reg.gather();
    let encoded = encoder.encode_to_string(&metrics_family).unwrap();

    encoded
}
