mod application_commands;
mod commands;
mod dbmodels;
mod mongo_conn;
mod startup;

use serenity::model::application::interaction::Interaction;
use songbird::SerenityInit;

use std::{
    env,
    sync::{
        Arc,
    },
};

use mongo_conn::get_mongo_client;
use serenity::{
    async_trait, framework::StandardFramework, model::prelude::GuildId, model::prelude::*,
    prelude::*,
};
use tracing::{error, info, warn};

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
        application_commands::handle_interactions(
            &_ctx,
            _interaction,
            &self.mongodb_client,
        )
        .await
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

    let handler = Handler {
        mongodb_client,
    };
    let intents = GatewayIntents::GUILD_INTEGRATIONS
    | GatewayIntents::GUILDS;
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
