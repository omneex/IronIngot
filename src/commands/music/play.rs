use serenity::async_trait;
use serenity::model::prelude::GuildId;
use songbird::id::GuildId as SongBirdGuildId;
use songbird::tracks::Queued;
use std::cell::Cell;
use std::fmt::Display;
use std::process::Stdio;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use tokio::process::Command as TokioCommand;

use serenity::model::application::command::Command as interaction_command;

use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;

use songbird::input::{ytdl_search, Metadata};
use songbird::{create_player, ytdl, Event, EventContext, EventHandler, TrackEvent, Songbird};
use tracing::{error, info};
use url::Url;

use crate::commands::common::interaction_error::interaction_error_edit;
use crate::commands::common::slash_commands::extract_vec;
use crate::mongo_conn::get_guild_doc;
enum QueryType {
    URL,
    PLAYLIST,
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
        Ok(url_obj) => {
            if let Some(playlist_param) = url_obj.query_pairs().find(|pair| pair.0 == "list") {
                QueryType::PLAYLIST
            } else {
                QueryType::URL
            }
        }
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
    let mut playlist: Vec<Metadata> = vec![];
    // Get the track
    let input_res = match query_type {
        QueryType::URL => ytdl(query_string).await,
        QueryType::SEARCH => ytdl_search(query_string).await,
        QueryType::PLAYLIST => {
            playlist = match ytdl_playlist(&query_string).await {
                Ok(playlist) => playlist,
                Err(e) => {
                    error!("{}", e);
                    interaction_error_edit("Failed to get the playlist.", interaction, ctx);
                    return;
                }
            };
            playlist.reverse();
            let meta = playlist.pop().unwrap();
            let new_query: String;
            let source_url: Url = match meta.source_url {
                Some(url_str) => Url::parse(&url_str).unwrap(),
                None => return,
            };

            ytdl(source_url.to_string()).await
        }
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

    // Queue the track
    let mut call = call_lock.lock().await;
    let (mut track, track_handle) = create_player(source);
    let guild_id_str = interaction.guild_id.unwrap().0.to_string();

    // Try to get the guild from the database, returns an option if the guild was found.
    let guild_doc = match get_guild_doc(mongo_client, guild_id_str, interaction, ctx).await {
        Some(value) => value,
        None => return,
    };

    track.set_volume(guild_doc.volume);

    call.enqueue(track);
    match query_type {
        QueryType::PLAYLIST => {
            call.add_global_event(
                Event::Track(TrackEvent::End),
                SongEndNotifier {
                    playlist: Mutex::new(playlist),
                    manager,
                    guild_id: guild.id,
                },
            );
        }
        _ => (),
    }
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

struct SongEndNotifier {
    playlist: Mutex<Vec<Metadata>>,
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

#[async_trait]
impl EventHandler for SongEndNotifier {
    async fn act(&self, _stx: &EventContext<'_>) -> Option<Event> {
        let video_id = match self.playlist.lock().unwrap().pop() {
            Some(meta) => match meta.source_url {
                Some(url_str) => url_str,
                None => return None,
            },
            None => return None,
        };
        let call_lock = self.manager.get(self.guild_id).unwrap();
        let mut call = call_lock.lock().await;
        let _ = call.queue().pause();
        let input = ytdl(video_id).await.unwrap();
        call.enqueue_source(input);
        call.queue().modify_queue(|queue| {
            // Make sure that the first 
            queue.swap(0, queue.len()-1)
        });
        let _ = call.queue().resume();
        None
    }
}

struct PlaylistError {
    cause: String,
}

impl Display for PlaylistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Extracting URLs from playlist failed: {}", self.cause)
    }
}

async fn ytdl_playlist(uri: &str) -> Result<Vec<Metadata>, PlaylistError> {
    let ytdl_args = [
        "--print-json",
        "--flat-playlist",
        "--ignore-config",
        "--no-warnings",
        uri,
    ];

    let youtube_dl_res = TokioCommand::new("yt-dlp")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    let youtube_dl_output = match youtube_dl_res {
        Ok(output) => match from_utf8(&output.stdout) {
            Ok(val) => val.to_owned(),
            Err(e) => {
                return Err(PlaylistError {
                    cause: e.to_string(),
                })
            }
        },
        Err(e) => {
            return Err(PlaylistError {
                cause: e.to_string(),
            })
        }
    };

    let mut metadata_vec = vec![];

    for el in youtube_dl_output.split('\n') {
        if el.is_empty() {
            continue;
        }
        let meta = match serde_json::from_str(&el.replace('\n', "")) {
            Ok(json) => Metadata::from_ytdl_output(json),
            Err(e) => {
                return Err(PlaylistError {
                    cause: e.to_string(),
                })
            }
        };
        metadata_vec.push(meta);
    }
    return Ok(metadata_vec);
}
