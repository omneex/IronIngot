
use std::process::Command;

use serenity::model::application::command::Command as interaction_command;

use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;

use songbird::input::ytdl_search;
use songbird::ytdl;
use tracing::{error, info};
use url::Url;

use crate::commands::common::interaction_error::interaction_error;
use crate::commands::common::slash_commands::extract_vec;
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

    let mut query_string_opt: Option<String> = None;
    for tup in extract_vec(&interaction.data.options).await {
        if tup.0 == "song" {
            if let Some(x) = super::super::common::slash_commands::get_string(tup.1) {
                query_string_opt = Some(x);
            } else {
                interaction_error("'song' param was invalid.", interaction, ctx).await;
                return;
            }
        }
    }

    let query_string = match query_string_opt {
        Some(x) => x,
        None => {
            interaction_error("'song' param was missing.", interaction, ctx).await;
            return;
        },
    };

    let query_type: QueryType = match Url::parse(&query_string) {
        Ok(_) => QueryType::URL,
        Err(_) => QueryType::SEARCH,
    };

    // Get the call
    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let guild = interaction.guild_id.unwrap().to_guild_cached(&ctx.cache).unwrap();
    let call_lock = match manager.get(guild.id) {
        Some(ongoing_call) => ongoing_call,
        None => {
            let voice_state = guild.voice_states.get(&interaction.user.id).unwrap();
            let vc = voice_state.channel_id.unwrap();
            let vc_name = vc.name(&ctx.cache).await.unwrap();
            manager.join(guild.id, vc).await.0
        },
    };

    // Get the track
    let input_res = match query_type {
        QueryType::URL => {
            ytdl(query_string).await
        },
        QueryType::SEARCH => {
            ytdl_search(query_string).await
        },
    };

    let source = match input_res {
        Ok(input ) => input,
        Err(err) => {
            error!("Error: {}", err);
            interaction_error("Failed to get the track.", interaction, ctx);
            return;
        },
    };

    let source_metadata = source.metadata.clone();
    info!("{:?}", source_metadata);
    
    // Queue the track
    let mut call = call_lock.lock().await;
    call.enqueue_source(source);
    let position: usize = call.queue().len();

    // Send the response
    info!("Creating response...");
    let _res = interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.embed(|embed| {
                        embed.title(format!("Queued Track: {}", position.to_string()));
                        embed.description(format!("[{}]({})", source_metadata.title.unwrap_or("NONE".to_string()), source_metadata.source_url.unwrap_or("NONE".to_string())));
                        embed.image(source_metadata.thumbnail.unwrap())
                    })
                })
        })
        .await;
    info!("Response created.");

}
#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = interaction_command::create_global_application_command(&*ctx.http, |command| {
        command.name("play").description("Adds a song to the queue.")
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