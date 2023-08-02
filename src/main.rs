use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[group]
#[commands(state)]
struct General;

struct Handler;

struct ClanState {}

struct ClanStates;
impl TypeMapKey for ClanStates {
    type Value = Arc<RwLock<HashMap<String, ClanState>>>;
}

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
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

    {
        let mut data = client.data.write().await;
        data.insert::<ClanStates>(Arc::new(RwLock::new(HashMap::new())));
    }

    tokio::spawn(async move {
        let key = tokio::fs::read_to_string("api.key").await.unwrap();
        let client = gold_pass_bot::Client::new(key);

        loop {
            if let Ok(w) = client.current_war("#2L99VLJ9P").await {
                dbg!(w);
            }
            if let Ok(w) = client.clan_war_league_group("#2L99VLJ9P").await {
                dbg!(&w);

                for round in w.rounds.iter() {
                    for tag in round.war_tags.iter() {
                        if tag.0.as_str() == "#0" {
                            continue;
                        }

                        dbg!(tag);
                        if let Ok(w) = client.clan_war_league_war(&tag).await {
                            dbg!(w);
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn state(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .send_message(&ctx.http, |m| m.content("Testing"))
        .await?;

    Ok(())
}
