use serenity::model::application::command::Command;

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

    let call = call_lock.lock().await;

    info!("Creating response...");
    let _res = interaction
        .edit_original_interaction_response(&ctx.http, |message| {
            message.embed(|embed| {
                embed.title("Current Queue");
                for (count, track) in call.queue().current_queue().iter().enumerate() {
                    if count > 25 {
                        break;
                    }
                    let title = track.metadata().title.clone().unwrap();
                    let source_url = track.metadata().source_url.clone().unwrap();
                    if count == 0 {
                        embed.field(
                            "Currently Playing",
                            format!("[{}]({})", title, source_url),
                            false,
                        );
                    } else {
                        embed.field(
                            (count + 1).to_string(),
                            format!("[{}]({})", title, source_url),
                            false,
                        );
                    }
                }
                embed
            });

            message
        })
        .await;
    info!("Response created.");
    
}
#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command.name("queue").description("The current queue.")
    })
    .await
    {
        error!("Could not register join command! {}", err.to_string());
        panic!()
    }
}
