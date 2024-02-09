mod application_commands;
mod commands;
mod dbmodels;
mod mongo_conn;
mod startup;

use mongodb::Collection;
use serenity::model::application::interaction::Interaction;
use songbird::SerenityInit;

use std::{env, sync::Arc};

use crate::dbmodels::guild::Guild as GuildStruct;
use mongo_conn::{get_collection, get_db, get_mongo_client};
use serenity::{
    async_trait, framework::StandardFramework, model::prelude::GuildId, model::prelude::*,
    prelude::*,
};
use tracing::{debug, error, info, warn};

use crate::startup::insert_guilds;

struct Handler {
    mongodb_client: mongodb::Client,
}

#[async_trait]
impl EventHandler for Handler {
    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        info!("Cache is ready, starting the redis-check-loop");
        let _ctx = Arc::new(ctx);

        let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
        let mongodb_client = match get_mongo_client(mongo_conn_str.as_str()).await {
            Ok(client) => client,
            Err(err) => {
                panic!("Could not get mongoDB client. {:?}", err)
            }
        };

        let _mongo_client = Arc::new(mongodb_client);

        // This is for if you want to run something else in a seperate thread
        // if !self.is_loop_running.load(Ordering::Relaxed) {
        //     info!("Starting the redis check loop");
        //     let ctx1 = Arc::clone(&ctx);
        //     let mongo_client1 = Arc::clone(&mongo_client);
        //     let redis_client1 = Arc::clone(&redis_client);

        //     tokio::spawn(async move {
        //         loop {
        //             check_redis(
        //                 Arc::clone(&ctx1),
        //                 Arc::clone(&mongo_client1),
        //                 Arc::clone(&redis_client1),
        //             )
        //             .await;
        //             tokio::time::sleep(Duration::from_secs(1)).await;
        //         }
        //     });

        //     // Now that the loop is running, we set the bool to true
        //     self.is_loop_running.swap(true, Ordering::Relaxed);
        // } else {
        //     debug!("Not running the loop because its already running.");
        // }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        // let clear_commands = false;
        // if clear_commands {
        //     application_commands::clear(&ctx).await;
        // }

        let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
        let client = match get_mongo_client(mongo_conn_str.as_str()).await {
            Ok(client) => client,
            Err(_) => {
                panic!("Could not get mongoDB client.")
            }
        };
        if let Err(err) = insert_guilds(&ctx, &client).await {
            warn!("{:?}", err)
        }

        application_commands::register(&ctx).await;
    }

    // Interaction handler
    async fn interaction_create(&self, _ctx: Context, _interaction: Interaction) {
        application_commands::handle_interactions(&_ctx, _interaction, &self.mongodb_client).await
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _new: bool) {
        let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
        let client = match get_mongo_client(mongo_conn_str.as_str()).await {
            Ok(client) => client,
            Err(_) => {
                panic!("Could not get mongoDB client.")
            }
        };
        let db = get_db(&client, "botdb").await;
        let col: Collection<GuildStruct> = get_collection(&db, "guilds", None).await;

        let guild_id_str = guild.id.0.to_string();

        let _ = col
            .insert_one(
                GuildStruct {
                    guild_ID: guild_id_str,
                    mod_channel_ID: "0".to_string(),
                    mod_role_ID: "0".to_string(),
                    prefix_string: "~".to_string(),
                    volume: 0.7,
                },
                None,
            )
            .await;
    }

    async fn voice_state_update(&self, ctx: Context, old_state_opt: Option<VoiceState>, new_state: VoiceState) {

        debug!("{:?}", new_state);

        let old_state = match old_state_opt {
            Some(state) => {
                info!("{:?}", state);
                info!("-->>");
                info!("{:?}", new_state);
                state
            },
            None => {
                info!("{:?}", new_state);
                return
            }, // They are joining, don't care
        };
        
        
        
        if new_state.channel_id.is_some() {
            return
        }

        let guild_channel: GuildChannel = match old_state.channel_id {
            Some(id) => {
                match id.to_channel(&ctx).await {
                    Ok(channel) => {
                        match channel.guild() {
                            Some(guild_channel) => guild_channel,
                            None => {
                                warn!("No guild for channel with new voice state");
                                return
                            },
                        } 
                    },
                    Err(_) => {
                        error!("Failed to get guild_channel from channel");
                        return
                    },
                }
            },
            None => {
                warn!("No channel id with new voice state");
                return
            },
        };

        let member_count = match guild_channel.members(&ctx).await {
            Ok(memebers) => memebers.len(),
            Err(_) => {
                error!("Failed to get memebers from guild_channel");
                return
            },
        };

        
        if member_count <= 0 {
            let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();
            
            let _ = manager.leave(guild_channel.guild_id).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt().json().init();
    info!("Starting the bot...");

    let framework = StandardFramework::new().configure(|c| c.prefix("~")); // set the bot's prefix to "~"

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Missing token in env!");

    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
    let mongodb_client = match get_mongo_client(mongo_conn_str.as_str()).await {
        Ok(client) => client,
        Err(err) => {
            panic!("Could not get mongoDB client. {:?}", err)
        }
    };

    let handler = Handler { mongodb_client };
    let intents = GatewayIntents::all();
    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .framework(framework)
        .register_songbird()
        .application_id(application_id)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("An error occurred while running the client: {:?}", why);
    }
}
