use anyhow::Result;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use songbird::input::reader::MediaSource;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, SerenityInit};
use std::env;
use std::io::Write;
use std::sync::Arc;
use tracing::{error, info};
use vosk::{Model, Recognizer};

/// Configuration loaded from environment variables
struct Config {
    discord_token: String,
    vosk_model_path: String,
    api_endpoint: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        Ok(Self {
            discord_token: env::var("DISCORD_TOKEN")?,
            vosk_model_path: env::var("VOSK_MODEL_PATH")?,
            api_endpoint: env::var("API_ENDPOINT")?,
        })
    }
}

/// Main event handler for Discord events
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

/// Audio receiver that transcribes audio using Vosk
struct Receiver {
    recognizer: Arc<Mutex<Recognizer>>,
    api_endpoint: String,
    channel_id: ChannelId,
}

impl Receiver {
    fn new(model: Arc<Model>, api_endpoint: String, channel_id: ChannelId) -> Self {
        let recognizer = Recognizer::new(&model, 48000.0).expect("Failed to create recognizer");
        Self {
            recognizer: Arc::new(Mutex::new(recognizer)),
            api_endpoint,
            channel_id,
        }
    }

    async fn send_to_api(&self, text: String, user_id: u64) -> Result<()> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "text": text,
            "user_id": user_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let response = client
            .post(&self.api_endpoint)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("API request failed: {}", response.status());
        }

        Ok(())
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::SpeakingStateUpdate(speaking) = ctx {
            if let Some(user_id) = speaking.user_id {
                info!("User {} started speaking", user_id);
            }
        }

        if let EventContext::VoicePacket { audio, packet, .. } = ctx {
            if let Some(audio_data) = audio {
                // Convert audio to PCM format expected by Vosk
                let mut recognizer = self.recognizer.lock().await;
                
                // Process audio data
                if recognizer.accept_waveform(audio_data) {
                    let result = recognizer.result();
                    
                    // Parse JSON result from Vosk
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result) {
                        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                info!("Transcription: {}", text);
                                
                                // Send to external API
                                if let Some(user_id) = packet.ssrc.map(|s| s as u64) {
                                    if let Err(e) = self.send_to_api(text.to_string(), user_id).await {
                                        error!("Failed to send to API: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

/// Join a voice channel and start transcribing
async fn join_channel(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    model: Arc<Model>,
    api_endpoint: String,
) -> Result<()> {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird not initialized")
        .clone();

    let (handler_lock, success) = manager.join(guild_id, channel_id).await;

    if success.is_ok() {
        let mut handler = handler_lock.lock().await;

        // Create receiver for audio processing
        let receiver = Receiver::new(model, api_endpoint, channel_id);

        // Register event handler
        handler.add_global_event(Event::Core(songbird::CoreEvent::SpeakingStateUpdate.into()), receiver.clone());
        handler.add_global_event(Event::Core(songbird::CoreEvent::VoicePacket.into()), receiver);

        info!("Joined channel {} in guild {}", channel_id, guild_id);
    } else {
        error!("Failed to join channel");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env()?;

    info!("Loading Vosk model from: {}", config.vosk_model_path);
    let model = Arc::new(Model::new(&config.vosk_model_path)?);
    info!("Model loaded successfully");

    // Create Discord client
    let intents = GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILD_MESSAGES;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await?;

    // Store model in client data for access in commands
    {
        let mut data = client.data.write().await;
        data.insert::<ModelKey>(model);
        data.insert::<ApiEndpointKey>(config.api_endpoint);
    }

    info!("Starting bot...");
    client.start().await?;

    Ok(())
}

// Type keys for storing data in client context
struct ModelKey;
impl TypeMapKey for ModelKey {
    type Value = Arc<Model>;
}

struct ApiEndpointKey;
impl TypeMapKey for ApiEndpointKey {
    type Value = String;
}
