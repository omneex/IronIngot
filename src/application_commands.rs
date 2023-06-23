use crate::commands::manage::*;
use crate::commands::misc::ping::command as pingcommand;
use crate::commands::music::join;
use crate::commands::music::leave;
use crate::commands::music::nowplaying;
use crate::commands::music::play;
use crate::commands::music::queue;
use crate::commands::music::skip;
use crate::commands::music::volume;
use mongodb::Client;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::command::Command;
use serenity::model::prelude::command::CommandPermissionType;
use serenity::model::prelude::interaction::Interaction;
use serenity::model::prelude::CommandId;
use serenity::model::prelude::CommandPermissionId;
use serenity::model::prelude::GuildId;
use serenity::model::prelude::RoleId;
use serenity::prelude::Context;
use tracing::*;

pub async fn register(ctx: &Context) {
    // Do all command registrations here.
    // If a command fails to register it will panic.
    info!("Registering commands...");
    setmodrole::register(ctx).await;
    join::register(ctx).await;
    leave::register(ctx).await;
    nowplaying::register(ctx).await;
    play::register(ctx).await;
    queue::register(ctx).await;
    skip::register(ctx).await;
    volume::register(ctx).await;
    info!("Done.");

    // Print out the currently registered commands.
    if let Err(err) = Command::get_global_application_commands(&*ctx.http)
        .await
        .map(|commands| {
            commands.iter().for_each(|command| {
                info!(
                    "Application command {} with ID {} is registered.",
                    command.name, command.id
                );
            })
        })
    {
        error!("Could not retrieve commands. {}", err.to_string())
    }
}

pub async fn handle_interactions(
    ctx: &Context,
    intn: Interaction,
    mongo_client: &mongodb::Client,
) {
    match intn {
        Interaction::Ping(_) => {}
        Interaction::ApplicationCommand(a_command) => {
            handle_commands(ctx, &a_command, mongo_client).await;
        }
        Interaction::MessageComponent(m_component) => {
            handle_components(&ctx, &m_component, mongo_client).await;
        }
        _ => {}
    }
}

async fn handle_commands(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    info!(
        "Application command '{}'({}) invoked by user '{}'({}) in Ch.{} Gld.{}",
        interaction.data.name,
        interaction.id.0,
        interaction.user.name,
        interaction.user.id,
        interaction.channel_id.0,
        interaction.guild_id.unwrap_or(GuildId(0))
    );

    match interaction.data.name.as_str() {
        "pingus" => {
            pingcommand(ctx, interaction, mongo_client).await;
        }
        "setmodrole" => {
            setmodrole::command(ctx, interaction, mongo_client).await;
        }
        "join" => {
            join::command(ctx, interaction, mongo_client).await;
        }
        "leave" => {
            leave::command(ctx, interaction, mongo_client).await;
        }
        "nowplaying" => {
            nowplaying::command(ctx, interaction, mongo_client).await;
        }
        "play" => {
            play::command(ctx, interaction, mongo_client).await;
        }
        "queue" => {
            queue::command(ctx, interaction, mongo_client).await;
        } 
        "skip" => {
            skip::command(ctx, interaction, mongo_client).await;
        }
        "volume" => {
            volume::command(ctx, interaction, mongo_client).await;
        }
        _ => {
            warn!("Command not found.");
        }
    };
}

async fn handle_components(
    _ctx: &&Context,
    m_component: &MessageComponentInteraction,
    _mongo_client: &Client,
) {
    let ids_split: Vec<&str> = m_component.data.custom_id.split(':').collect();
    let _comp_type: &str = match ids_split.first() {
        Some(str_type) => str_type,
        None => "none",
    };
    // TODO possibly avoid another split here by using this split again, but for now I dont want to edit the signiture
    {
        warn!("Interaction not found.");
    }
}

// pub async fn clear(ctx: &Context) {
//     info!("Clearing slash commands...");
//     let mut commands_to_del: Vec<(CommandId, String)> = vec![];
//     let _res = Command::get_global_application_commands(&*ctx.http)
//         .await
//         .map(|comms| {
//             comms.iter().for_each(|comm| {
//                 let name = comm.name.clone();
//                 let id = comm.id.clone();
//                 commands_to_del.push((id, name))
//             })
//         })
//     info!(
//         "There are {} command/s to be cleared.",
//         commands_to_del.len()
//     );
//     for x in 0..commands_to_del.len() {
//         info!(
//             "Deleting command '{}' with ID {}",
//             commands_to_del[x].1, commands_to_del[x].0
//         );
//         let _res =
//             Command::delete_global_application_command(&*ctx.http, commands_to_del[x].0)
//                 .await;
//     }
//     for guild in ctx.cache.guilds().await {
//         let commands = match ctx.http.get_guild_application_commands(guild.0).await {
//             Ok(commands) => commands,
//             Err(e) => panic!("{}",e)
//         };
//         for command in commands {
//             match ctx.http.delete_guild_application_command(guild.0, command.id.0).await {
//                 Ok(_) => {}
//                 Err(e) => panic!("{}", e)
//             }
//         };
//     }
//     info!("Commands cleared. Will now re-add commands.");
// }

#[instrument(skip(ctx, command))]
pub async fn add_admins_to_perms(
    ctx: &Context,
    command: Command,
    guild_id: GuildId,
) -> serenity::static_assertions::_core::result::Result<(), &'static str> {
    let mut admin_role_ids: Vec<RoleId> = vec![];
    // Get roles with admin
    match guild_id.roles(&*ctx.http).await {
        Ok(role_map) => {
            for role_tup in role_map {
                let (role_id, role) = role_tup;
                if role.permissions.administrator() && role.tags.bot_id.is_none() {
                    admin_role_ids.push(role_id);
                }
            }
        }
        Err(_) => {
            error!("Could not retrieve the guild roles.");
            return Err("Could not retrieve the guild roles.");
        }
    };
    for id in &admin_role_ids {
        match guild_id
            .create_application_command_permission(&*ctx.http, command.id, |perms| {
                perms.create_permission(|perm_data| {
                    perm_data
                        .id(id.0)
                        .permission(true)
                        .kind(CommandPermissionType::Role)
                })
            })
            .await
        {
            Ok(_) => {}
            Err(_) => {
                error!("Failed to create perm.");
                return Err("Failed to create perm.");
            }
        }
    }

    let perm = guild_id
        .get_application_command_permissions(&ctx.http, command.id)
        .await;
    match perm {
        Ok(_) => {}
        Err(_) => {
            return Err("Failed to get permissions.");
        }
    }
    Ok(())
}

#[instrument(skip(ctx))]
pub async fn get_vec_of_perms(
    ctx: &Context,
    command_id: &CommandId,
    guild_id: &GuildId,
) -> serenity::static_assertions::_core::result::Result<
    Vec<(CommandPermissionId, bool)>,
    &'static str,
> {
    let mut vec_of_roles: Vec<(CommandPermissionId, bool)> = vec![];
    let perm = guild_id
        .get_application_command_permissions(&ctx.http, *command_id)
        .await;
    match perm {
        Ok(p) => {
            for pe in p.permissions {
                vec_of_roles.push((pe.id, pe.permission));
            }
        }
        Err(_) => {
            return Err("Failed to get permissions.");
        }
    }
    Ok(vec_of_roles)
}
