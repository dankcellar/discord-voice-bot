pub mod receiver;

use songbird::Call;
use tokio::sync::MutexGuard;
use crate::voice::receiver::VoiceReceiver;

pub async fn subscribe_to_audio(handler: &mut MutexGuard<'_, Call>) {
    // Determine how to attach.
    // In Songbird 0.4, we can register an event handler for VoicePacket.
    // However, handling raw packets requires managing the jitter buffer, etc.
    // Since the prompt asks for "Refactor", and the original code did it manually,
    // we can either do it manually or use Songbird's receiver.
    
    // We'll use the event system to intercept packets.
    let receiver = VoiceReceiver::new();
    
    handler.add_global_event(
        songbird::CoreEvent::VoicePacket.into(),
        receiver
    );
}
