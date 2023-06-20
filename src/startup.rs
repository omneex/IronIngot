use crate::dbmodels::guild::Guild;
use crate::mongo_conn::{get_collection, get_db};
use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use mongodb::*;
use serenity::prelude::*;
use tracing::*;

#[instrument(skip(ctx, client))]
pub async fn insert_guilds(ctx: &Context, client: &mongodb::Client) -> Result<(), String> {
    let db = get_db(client, "botdb").await;
    let col: Collection<Guild> = get_collection(&db, "guilds", None).await;
    let guilds = ctx.cache.guilds();
    for guild in guilds {
        info!("Inserting ({}) into MongoDB", guild.0);
        let res = col
            .insert_one(
                Guild {
                    guild_ID: guild.0.to_string(),
                    mod_channel_ID: "0".to_string(),
                    mod_role_ID: "0".to_string(),
                    prefix_string: "~".to_string(),
                    volume: 0.7
                },
                None,
            )
            .await;

        if let Err(err) = res {
            return Err(format!("{:?}", err));
        }

        let model = IndexModel::builder()
            .keys(doc! {"guild_ID": 1})
            .options(IndexOptions::builder().unique(true).build())
            .build();

        let res = col.create_index(model, None).await;

        match res {
            Ok(_) => {}
            Err(e) => {
                error!("{:?}", e)
            }
        }
    }
    Ok(())
}
