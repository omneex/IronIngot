use mongodb::bson::doc;
use mongodb::Collection;

use serenity::model::prelude::command::Command;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{debug, error, info};

use crate::commands::common::interaction_error::interaction_error_edit;
use crate::commands::common::slash_commands::extract_vec;
use crate::dbmodels::guild::Guild as GuildStruct;

#[allow(unused)]
pub async fn command(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    interaction
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await;

    let options = interaction.data.options.clone();
    let mut float_opt: Option<f32> = None;
    let mut int_opt: Option<i64> = None;
    for tup in extract_vec(&options).await {
        if tup.0 == "volume" {
            if let Some(x) = super::super::common::slash_commands::get_int(tup.1) {
                float_opt = Some(x as f32 / 100.0);
                int_opt = Some(x);
            } else {
                interaction_error_edit("'volume' param was invalid.", interaction, ctx).await;
                return;
            }
        }
    }

    let volume_int = match int_opt {
        Some(val) => val,
        None => {
            interaction_error_edit("'volume' param was missing.", interaction, ctx).await;
            return;
        }
    };

    let volume = match float_opt {
        Some(val) => val,
        None => {
            interaction_error_edit("'volume' param was missing.", interaction, ctx).await;
            return;
        }
    };

    let guild = interaction
        .guild_id
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild.id) {
        let mut handler = handler_lock.lock().await;

        match handler.queue().current() {
            Some(track_handle) => {
                track_handle.set_volume(volume);
            }
            None => {
                debug!("No track, wont set volume there.")
            }
        }

        info!("Creating response...");
        let _res = interaction
            .edit_original_interaction_response(&ctx.http, |message| {
                message.embed(|embed| {
                    embed.title("Volume Changed");
                    embed.description("Set volume to ");
                    embed
                });
                message
            })
            .await;
        info!("Response created.");

        let guild_id_str = guild.id.0.to_string();
        let collection: Collection<GuildStruct> =
            mongo_client.database("botdb").collection("guilds");
        let update_res = match collection
            .update_one(
                doc! {"guild_ID": guild_id_str},
                doc! {"$set": {"volume": volume}},
                None,
            )
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("{:?}", err);
                interaction_error_edit("Could not update the database.", interaction, ctx).await;
                return;
            }
        };
    } else {
        interaction_error_edit("Something went wrong!", interaction, ctx).await;
    }
}

#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("volume")
            .description("Changes the volume.")
            .create_option(|option| {
                option.max_int_value(100);
                option.min_int_value(0);
                option.kind(serenity::model::prelude::command::CommandOptionType::Integer);
                option.name("volume")
            })
    })
    .await
    {
        error!("Could not register nowplaying command! {}", err.to_string());
        panic!()
    }
}
