use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::command::Command;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info};

use crate::commands::common::interaction_error::interaction_error_edit;

#[allow(unused)]
pub async fn command(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
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
        let queue = handler.queue().current_queue();
        let track_handle = match queue.get(0) {
            None => {
                info!("Creating response...");
                let _res = interaction
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.flags(MessageFlags::EPHEMERAL);
                                message.embed(|embed| {
                                    embed.title("Now Playing");
                                    embed.description("There is nothing playing right now...");
                                    embed.footer(|footer| {
                                        footer.text("Queue position 0 is empty.");
                                        footer
                                    });
                                    embed
                                });
                                message
                            })
                    })
                    .await;
                info!("Response created.");
                return;
            }
            Some(track_handle) => track_handle,
        };

        if let Err(track_error) = handler.queue().skip() {
            error!("{}", track_error.to_string());
            interaction_error_edit("Failed to skip song!", interaction, ctx);
            return ;
        };

        info!("Creating response...");
        let _res = interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|embed| {
                            embed.title("Skipped!");

                            if let Some(track_title) = &track_handle.metadata().title {
                                embed.description(track_title);
                            }

                            if let Some(source_url) = &track_handle.metadata().source_url {
                                embed.url(source_url);
                            }

                            if let Some(thumbnail_url) = &track_handle.metadata().thumbnail {
                                embed.image(thumbnail_url);
                            }

                            embed
                        });
                        message
                    })
            })
            .await;
        info!("Response created.");
    } else {
        interaction_error_edit("Something went wrong!", interaction, ctx).await;
    }
}

#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("skip")
            .description("Skips the current song playing")
    })
    .await
    {
        error!("Could not register nowplaying command! {}", err.to_string());
        panic!()
    }
}
