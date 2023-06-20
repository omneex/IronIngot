use crate::commands::common::interaction_error::interaction_error;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::command::*;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::*;
use tracing::{error, info};

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
    let voice_state = guild.voice_states.get(&interaction.user.id).unwrap();
    let vc = voice_state.channel_id.unwrap();
    let vc_name = vc.name(&ctx.cache).await.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let has_handler = manager.get(guild.id).is_some();
    if has_handler {
        if let Err(e) = manager.remove(guild.id).await {
            interaction_error("Failed to leave, try again in a moment.", interaction, ctx).await;
        }

        info!("Creating response...");
        let _res = interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.flags(MessageFlags::EPHEMERAL);
                        message.content(format!("Left {}.", vc_name))
                    })
            })
            .await;
        info!("Response created.");
    } else {
        info!("Creating response...");
        let _res = interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.flags(MessageFlags::EPHEMERAL);
                        message.content("Bot is not in a voice chat.".to_string())
                    })
            })
            .await;
        info!("Response created.");
    }
}
#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("leave")
            .description("Leaves the Voice Chat that the bot is currently connected to.")
    })
    .await
    {
        error!("Could not register leave command! {}", err.to_string());
        panic!()
    }
}
