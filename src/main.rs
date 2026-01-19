mod config;
mod discord_bot;
mod transcription;
mod vosk_model;

use config::Config;
use discord_bot::{Handler, VoskModelKey};
use serenity::client::Client;
use serenity::prelude::*;
use songbird::SerenityInit;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let model = vosk_model::load()?;

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await?;

    client.data.write().await.insert::<VoskModelKey>(model);
    client.start().await?;

    Ok(())
}
