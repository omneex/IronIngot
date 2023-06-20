use serenity::model::application::command::Command as interaction_command;

use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;

use songbird::input::ytdl_search;
use songbird::{create_player, ytdl};
use tracing::{error, info};
use url::Url;

use crate::commands::common::interaction_error::interaction_error_edit;
use crate::commands::common::slash_commands::extract_vec;
use crate::mongo_conn::get_guild_doc;
enum QueryType {
    URL,
    SEARCH,
}

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

    let mut query_string_opt: Option<String> = None;
    for tup in extract_vec(&interaction.data.options).await {
        if tup.0 == "song" {
            if let Some(x) = super::super::common::slash_commands::get_string(tup.1) {
                query_string_opt = Some(x);
            } else {
                interaction_error_edit("'song' param was invalid.", interaction, ctx).await;
                return;
            }
        }
    }

    let query_string = match query_string_opt {
        Some(x) => x,
        None => {
            interaction_error_edit("'song' param was missing.", interaction, ctx).await;
            return;
        }
    };

    let query_type: QueryType = match Url::parse(&query_string) {
        Ok(_) => QueryType::URL,
        Err(_) => QueryType::SEARCH,
    };

    // Get the call
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let guild = interaction
        .guild_id
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .unwrap();
    let call_lock = match manager.get(guild.id) {
        Some(ongoing_call) => ongoing_call,
        None => {
            let voice_state = guild.voice_states.get(&interaction.user.id).unwrap();
            let vc = voice_state.channel_id.unwrap();
            let vc_name = vc.name(&ctx.cache).await.unwrap();
            manager.join(guild.id, vc).await.0
        }
    };

    // Get the track
    let input_res = match query_type {
        QueryType::URL => ytdl(query_string).await,
        QueryType::SEARCH => ytdl_search(query_string).await,
    };

    let source = match input_res {
        Ok(input) => input,
        Err(err) => {
            error!("Error: {}", err);
            interaction_error_edit("Failed to get the track.", interaction, ctx).await;
            return;
        }
    };

    let source_metadata = source.metadata.clone();
    info!("{:?}", source_metadata);

    // Queue the track
    let mut call = call_lock.lock().await;
    let (mut audio, audio_handle) = create_player(source);
    let guild_id_str = interaction.guild_id.unwrap().0.to_string();

    // Try to get the guild from the database, returns an option if the guild was found.
    let guild_doc = match get_guild_doc(mongo_client, guild_id_str, interaction, ctx).await {
        Some(value) => value,
        None => return,
    };

    audio.set_volume(guild_doc.volume);

    call.enqueue(audio);
    let position: usize = call.queue().len();

    // Send the response
    info!("Creating response...");
    let _res = interaction
        .edit_original_interaction_response(&ctx.http, |message| {
            message.embed(|embed| {
                embed.title(format!("Queued Track: {}", position));
                embed.description(format!(
                    "[{}]({})",
                    source_metadata.title.unwrap_or("NONE".to_string()),
                    source_metadata.source_url.unwrap_or("NONE".to_string())
                ));
                embed.image(source_metadata.thumbnail.unwrap())
            })
        })
        .await;
    info!("Response created.");
}

#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) =
        interaction_command::create_global_application_command(&*ctx.http, |command| {
            command
                .name("play")
                .description("Adds a song to the queue.")
                .create_option(|opt| {
                    opt.name("song")
                        .description("A URL or search query.")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        .await
    {
        error!("Could not register join command! {}", err.to_string());
        panic!()
    }
}
