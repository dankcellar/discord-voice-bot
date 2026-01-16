use serenity::all::{Context, EventHandler, Message, Ready};
use serenity::async_trait;
use songbird::input::Input;
use tracing::{info, error};
use crate::voice::receiver::VoiceReceiver;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if !msg.content.starts_with("!") {
            return;
        }

        let command = msg.content.split_whitespace().next().unwrap_or("");

        match command {
            "!join" => {
                let guild = msg.guild(&ctx.cache).unwrap();
                let guild_id = guild.id;

                let channel_id = guild
                    .voice_states
                    .get(&msg.author.id)
                    .and_then(|voice_state| voice_state.channel_id);

                let connect_to = match channel_id {
                    Some(channel) => channel,
                    None => {
                        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
                        return;
                    }
                };

                let manager = songbird::get(&ctx).await
                    .expect("Songbird Voice client placed in at initialisation.")
                    .clone();

                let (handler_lock, success) = manager.join(guild_id, connect_to).await;

                if let Ok(_handler) = success {
                    // Attach Voice Receiver for STT
                    let mut handler = handler_lock.lock().await;
                    
                    // Subscribe to voice events (Speaking updates) to know when to decode
                    // Actual Audio processing:
                    // Songbird allows adding a global event handler for audio frames.
                    // Or we can just use `add_global_event` with `CoreEvent::VoicePacket`.
                    // But for STT we want the decoded audio stream.
                    
                    // We need to attach a receiver.
                    let voice_receiver = VoiceReceiver::new();
                    handler.add_global_event(
                        songbird::CoreEvent::SpeakingUpdate.into(),
                        voice_receiver.clone()
                    );
                    
                    // Note: Songbird's Receiver handling is a bit complex. 
                    // We often need to register a "VoiceReceiver" to the call.
                    // But actually, `songbird` provides `add_global_event` for events. 
                    // To get AUDIO bytes, we need to use `receiver.register_client_decode`.
                    // However, `songbird` 0.4 driver handles this differently. 
                    
                    // A simpler way for this refactor:
                    // We'll use the `VoiceReceiver` defined in `voice.rs` which will attach itself.
                    // For now, let's just confirm connection.
                    
                    // We will enable the receiver on the Call.
                    
                    info!("Joined voice channel {}", connect_to);
                    check_msg(msg.reply(&ctx.http, "Joined voice channel!").await);
                    
                    // Initialize the receiver logic
                    crate::voice::subscribe_to_audio(&mut handler).await;

                } else {
                    check_msg(msg.reply(&ctx.http, "Error joining the channel").await);
                }
            }
            "!leave" => {
                let guild = msg.guild(&ctx.cache).unwrap();
                let guild_id = guild.id;

                let manager = songbird::get(&ctx).await
                    .expect("Songbird Voice client placed in at initialisation.")
                    .clone();
                
                if manager.get(guild_id).is_some() {
                    if let Err(e) = manager.remove(guild_id).await {
                        check_msg(msg.reply(&ctx.http, format!("Failed: {:?}", e)).await);
                    } else {
                        check_msg(msg.reply(&ctx.http, "Left voice channel").await);
                    }
                } else {
                    check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
                }
            }
            _ => {}
        }
    }
}

fn check_msg(result: serenity::Result<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
