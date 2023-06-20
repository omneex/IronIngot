use serenity::model::prelude::command::Command;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info};

#[allow(unused)]
pub async fn command(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    interaction
        .create_interaction_response(&ctx.http, |response| {
            response.interaction_response_data(|message| message.ephemeral(true));
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await;

    let guild = interaction
        .guild_id
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .unwrap();
    let voice_state = guild.voice_states.get(&interaction.user.id).unwrap();
    let vc = voice_state.channel_id.unwrap();
    let vc_name = vc.name(&ctx.cache).await.unwrap();

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
                    .edit_original_interaction_response(&ctx.http, |message| {
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
                    .await;

                info!("Response created.");
                return;
            }
            Some(track_handle) => track_handle,
        };

        info!("Creating response...");
        let _res = interaction
            .edit_original_interaction_response(&ctx.http, |message| {
                message.embed(|embed| {
                    embed.title("Now Playing");

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
            .await;
        info!("Response created.");
    } else {
    }
}

#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("nowplaying")
            .description("Displays the currently playing audio.")
    })
    .await
    {
        error!("Could not register nowplaying command! {}", err.to_string());
        panic!()
    }
}
