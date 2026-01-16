use dotenv::dotenv;
use serenity::all::GatewayIntents;
use serenity::client::Client;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use songbird::SerenityInit;
use tracing::{error, info};

mod commands;
mod control;
mod voice;

#[tokio::main]
async fn main() {
    // Load .env
    dotenv().ok();
    
    // Setup logging
    tracing_subscriber::fmt::init();
    info!("Starting Discord Voice Bot...");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    
    // Check for Vosk model path
    let model_path = env::var("VOSK_MODEL_PATH").unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());
    if !std::path::Path::new(&model_path).exists() {
        tracing::warn!("Vosk model not found at {}. Please run setup.sh or set VOSK_MODEL_PATH.", model_path);
    }

    // Configure intents
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES;

    // Build client
    let mut client = Client::builder(&token, intents)
        .event_handler(commands::Handler)
        .register_songbird()
        .await
        .expect("Err creating client");

    // Shared state between HTTP server and Bot
    let http_client = client.http.clone();
    let cache = client.cache.clone();
    let manager = songbird::get(&client).await.expect("Songbird Voice client placed in at initialisation.").clone();

    // Spawn Control Server
    let control_port = env::var("CONTROL_PORT").unwrap_or_else(|_| "3000".to_string()).parse::<u16>().unwrap_or(3000);
    tokio::spawn(async move {
        control::start_server(control_port, manager, http_client, cache).await;
    });

    // Start Bot
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
