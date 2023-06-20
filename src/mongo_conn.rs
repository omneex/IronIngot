use std::env;

use crate::commands::common::interaction_error::interaction_error_edit;
use crate::dbmodels::guild::Guild as GuildStruct;
use mongodb::bson::doc;
use mongodb::options::{ClientOptions, CollectionOptions, ResolverConfig};
use mongodb::*;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::Context;
use tracing::{error};

pub async fn get_mongo_client(connection_str: &str) -> mongodb::error::Result<Client> {
    let platform = env::var("PLATFORM").expect("No PLATFORM env set.");

    match platform.as_str() {
        "windows" => {
            let client_options = ClientOptions::parse_with_resolver_config(
                connection_str,
                ResolverConfig::cloudflare(),
            )
            .await?;
            Client::with_options(client_options)
        }
        "linux" => {
            let client_options = ClientOptions::parse(connection_str).await?;
            Client::with_options(client_options)
        }
        _ => {
            panic!("Invalid PLATFORM env value.")
        }
    }
}

pub async fn get_db(client: &Client, db_name: &str) -> Database {
    client.database(db_name)
}

pub async fn get_collection<T>(
    db: &Database,
    collection_str: &str,
    options: Option<CollectionOptions>,
) -> Collection<T> {
    let col: Collection<T> = match options {
        None => db.collection(collection_str),
        Some(options) => db.collection_with_options(collection_str, options),
    };
    col
}

pub async fn get_guild_doc(
    mongo_client: &mongodb::Client,
    guild_id_str: String,
    interaction: &ApplicationCommandInteraction,
    ctx: &Context,
) -> Option<GuildStruct> {
    let guild_doc_opt: Option<GuildStruct> = match mongo_client
        .database("botdb")
        .collection("guilds")
        .find_one(doc! {"guild_ID": guild_id_str}, None)
        .await
    {
        Ok(col_opt) => match col_opt {
            Some(col) => col,
            None => {
                interaction_error_edit("Guild is not in database", interaction, ctx).await;
                return None;
            }
        },
        Err(err) => {
            error!("{:?}", err);
            interaction_error_edit("Database error", interaction, ctx).await;
            return None;
        }
    };
    let guild_doc: GuildStruct = match guild_doc_opt {
        Some(doc) => doc,
        None => {
            interaction_error_edit("Guild is not in database", interaction, ctx).await;
            return None;
        }
    };
    Some(guild_doc)
}
