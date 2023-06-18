
use serenity::model::application::command::Command;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info};

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



    info!("Creating response...");
    let _res = interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.content(format!("Joining {}...", vc_name))
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
        command.name("join").description("Joins the Voice Chat that you are in.")
    })
        .await
    {
        error!("Could not register join command! {}", err.to_string());
        panic!()
    }
}