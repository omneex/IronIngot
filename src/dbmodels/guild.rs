use serde::*;

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Guild {
    pub guild_ID: String,
    pub mod_channel_ID: String,
    pub mod_role_ID: String,
    pub prefix_string: String,
    pub volume: f32
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SocialMediaAccounts {
    pub account_type: String,
    pub account_ID: String,
    pub discord_ID: String,
}
