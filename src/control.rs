use warp::Filter;
use std::sync::Arc;
use songbird::Songbird;
use serenity::http::Http;
use serenity::cache::Cache;
use serde::Deserialize;
use serenity::model::id::{GuildId, ChannelId};
use tracing::{info, error};

#[derive(Deserialize)]
struct ControlCommand {
    #[serde(rename = "type")]
    cmd_type: String, // "join" or "leave"
    #[serde(rename = "guildId")]
    guild_id: u64,
    #[serde(rename = "channelId")]
    channel_id: Option<u64>,
}

pub async fn start_server(
    port: u16,
    manager: Arc<Songbird>,
    http: Arc<Http>,
    cache: Arc<Cache>
) {
    let manager = warp::any().map(move || manager.clone());
    let http = warp::any().map(move || http.clone());
    
    // Health check
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
             warp::reply::json(&serde_json::json!({ "status": "ok", "vosk": true }))
        });

    // Control endpoint
    let control = warp::path("control")
        .and(warp::post())
        .and(warp::body::json())
        .and(manager)
        .and(http)
        .then(|cmd: ControlCommand, manager: Arc<Songbird>, _http: Arc<Http>| async move {
            match cmd.cmd_type.as_str() {
                "join" => {
                    if let Some(cid) = cmd.channel_id {
                        let guild_id = GuildId::new(cmd.guild_id);
                        let channel_id = ChannelId::new(cid);
                        
                        let (handler_lock, success) = manager.join(guild_id, channel_id).await;
                        if let Ok(_handler) = success {
                            let mut handler = handler_lock.lock().await;
                             crate::voice::subscribe_to_audio(&mut handler).await;
                            info!("Control: Joined {}", cid);
                            return warp::reply::json(&serde_json::json!({ "success": true }));
                        }
                    }
                    warp::reply::json(&serde_json::json!({ "success": false, "error": "Join failed" }))
                },
                "leave" => {
                    let guild_id = GuildId::new(cmd.guild_id);
                    if let Err(e) = manager.remove(guild_id).await {
                         error!("Control: Leave failed: {:?}", e);
                         return warp::reply::json(&serde_json::json!({ "success": false, "error": "Leave failed" }));
                    }
                    info!("Control: Left guild {}", cmd.guild_id);
                    warp::reply::json(&serde_json::json!({ "success": true }))
                },
                _ => warp::reply::json(&serde_json::json!({ "success": false, "error": "Unknown command" }))
            }
        });

    let routes = health.or(control);

    info!("Control server listening on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
