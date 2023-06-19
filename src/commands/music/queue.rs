
use serenity::model::application::command::Command;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info};

use crate::commands::music::queue;

#[allow(unused)]
pub async fn command(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    let guild = interaction.guild_id.unwrap().to_guild_cached(&ctx.cache).unwrap();
    let voice_state = guild.voice_states.get(&interaction.user.id).unwrap();
    let vc = voice_state.channel_id.unwrap();
    let vc_name = vc.name(&ctx.cache).await.unwrap();

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

    let call = call_lock.lock().await;
    

    info!("Creating response...");
    let _res = interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.embed(|embed| {
                        embed.title("Current Queue");
                        let mut count = 0;
                        for track in &call.queue().current_queue() {
                            if count > 25 {
                                break;
                            }
                            let title = track.metadata().title.clone().unwrap();
                            let source_url = track.metadata().source_url.clone().unwrap();
                            if count == 0 {
                                embed.field("Currently Playing", format!("[{}]({})", title, source_url), false);
                            } else {
                                embed.field((count+1).to_string(), format!("[{}]({})", title, source_url), false);
                            }
                            count+=1;
                        };
                        embed
                    });
                    
                    message
                })
        })
        .await;
    info!("Response created.");

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    manager.join(guild.id, vc).await;

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