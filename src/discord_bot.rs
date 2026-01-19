use crate::transcription::Receiver;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::Event;
use std::sync::Arc;
use vosk::Model;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, _ready: Ready) {}

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        match msg.content.as_str() {
            "!join" => {
                let channel_id_opt = if let Some(guild_id) = msg.guild_id {
                    let guild = ctx.cache.guild(guild_id);
                    if let Some(guild_ref) = guild {
                        guild_ref
                            .voice_states
                            .get(&msg.author.id)
                            .and_then(|vs| vs.channel_id)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(guild_id) = msg.guild_id {
                    if let Some(channel_id) = channel_id_opt {
                        let data = ctx.data.read().await;

                        let model = match data.get::<VoskModelKey>() {
                            Some(m) => m.clone(),
                            None => {
                                let _ = msg
                                    .reply(&ctx.http, "❌ Bot not properly initialized")
                                    .await;
                                return;
                            }
                        };

                        drop(data);

                        let join_result =
                            join_voice_channel(&ctx, guild_id, channel_id, model).await;
                        match join_result {
                            Ok(_) => {
                                let _ = msg.reply(&ctx.http, "✅ Joined your voice channel!").await;
                            }
                            Err(e) => {
                                let error_msg = format!("❌ Failed to join: {}", e);
                                let _ = msg.reply(&ctx.http, error_msg).await;
                            }
                        }
                    } else {
                        let _ = msg
                            .reply(&ctx.http, "❌ You must be in a voice channel first!")
                            .await;
                    }
                } else {
                    let _ = msg
                        .reply(&ctx.http, "❌ This command must be used in a server!")
                        .await;
                }
            }
            "!leave" => {
                if let Some(guild_id) = msg.guild_id {
                    let manager = songbird::get(&ctx).await.expect("Songbird not initialized");

                    match manager.remove(guild_id).await {
                        Ok(_) => {
                            let _ = msg.reply(&ctx.http, "👋 Left the voice channel!").await;
                        }
                        Err(_) => {
                            let _ = msg
                                .reply(&ctx.http, "❌ Failed to leave voice channel!")
                                .await;
                        }
                    }
                } else {
                    let _ = msg
                        .reply(&ctx.http, "❌ This command must be used in a server!")
                        .await;
                }
            }
            _ => {}
        }
    }
}

pub async fn join_voice_channel(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    model: Arc<Model>,
) -> Result<(), String> {
    let manager = songbird::get(ctx).await.expect("Songbird not initialized");

    let handler_lock = manager
        .join(guild_id, channel_id)
        .await
        .map_err(|e| format!("Join error: {:?}", e))?;

    let mut handler = handler_lock.lock().await;
    handler.remove_all_global_events();

    let receiver = Receiver::new(model);

    handler.add_global_event(
        Event::Core(songbird::CoreEvent::SpeakingStateUpdate.into()),
        receiver.clone(),
    );
    handler.add_global_event(
        Event::Core(songbird::CoreEvent::ClientDisconnect.into()),
        receiver.clone(),
    );
    handler.add_global_event(
        Event::Core(songbird::CoreEvent::DriverDisconnect.into()),
        receiver.clone(),
    );
    handler.add_global_event(Event::Core(songbird::CoreEvent::VoiceTick.into()), receiver);

    Ok(())
}

pub struct VoskModelKey;
impl TypeMapKey for VoskModelKey {
    type Value = Arc<Model>;
}
