use std::sync::{Arc, Mutex}; // Use std blocking mutex for simple state or tokio
use std::collections::HashMap;
use songbird::{Event, EventContext, EventHandler as SongbirdEventHandler};
use audiopus::{coder::Decoder, Channels, SampleRate};
use vosk::{Model, Recognizer};
use tokio::sync::mpsc;

// We need a global model reference or per-decoder. 
// Vosk Model is thread-safe (Arc).
// Recognizer is not, needed per user.

lazy_static::lazy_static! {
    static ref VOSK_MODEL: Arc<Model> = {
        let path = std::env::var("VOSK_MODEL_PATH").unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());
        Arc::new(Model::new(path).expect("Could not load Vosk model"))
    };
}

// User audio stream state
struct UserStream {
    decoder: Decoder,
    recognizer: Recognizer,
    buffer: Vec<i16>, // Accumulate PCM
}

impl UserStream {
    fn new(model: Arc<Model>) -> Self {
        // Discord: 48kHz, Stereo
        let decoder = Decoder::new(SampleRate::Hz48000, Channels::Stereo).expect("Failed to create Opus decoder");
        
        // Vosk: 16kHz, Mono (usually)
        let recognizer = Recognizer::new(&model, 16000.0).expect("Failed to create Recognizer");
        
        Self {
            decoder,
            recognizer,
            buffer: Vec::new(),
        }
    }

    fn process_packet(&mut self, payload: &[u8]) {
        // 1. Decode Opus -> PCM (48kHz Stereo)
        let mut pcm_out = [0i16; 5760]; // Max frame size for 120ms at 48kHz? Usually 20ms = 960 samples * 2 channels = 1920
        // Standard Discord frame is 20ms (960 samples).
        
        match self.decoder.decode(Some(payload), &mut pcm_out, false) {
            Ok(samples) => {
                let decoded_slice = &pcm_out[..samples * 2]; // *2 for stereo
                
                // 2. Resample / Mix to Mono 16kHz
                // Simple decimation: Take average of L/R, then skip samples.
                // 48k -> 16k is factor of 3.
                
                for i in (0..decoded_slice.len()).step_by(6) { // 2 channels * 3 step = 6
                     if i + 1 < decoded_slice.len() {
                         let left = decoded_slice[i] as i32;
                         let right = decoded_slice[i+1] as i32;
                         let mono = ((left + right) / 2) as i16;
                         
                         self.buffer.push(mono);
                     }
                }
                
                // 3. Feed to Vosk if buffer is large enough
                // Vosk implementation in Rust takes &[i16] directly via `accept_waveform`
                if self.buffer.len() > 4000 { // Approx 250ms of 16kHz
                    let _ = self.recognizer.accept_waveform(&self.buffer);
                    self.buffer.clear();
                    
                    // Check results
                    let result = self.recognizer.partial_result(); // or result()
                     // We print partials or finals
                    if let Some(txt) = result.partial {
                        if !txt.is_empty() {
                            // Don't spam partials too much
                            // tracing::info!("Partial: {}", txt);
                        }
                    }
                     // Usually we need to check `final_result` logic, but `result()` clears context.
                     // The original node bot used `result()`. 
                }
                
                // Note: Real usage requires logic to detect "end of utterance" to call result().
                // For this refactor, we stick to accept_waveform logic.
            }
            Err(e) => tracing::error!("Opus decode error: {:?}", e),
        }
    }
}

#[derive(Clone)]
pub struct VoiceReceiver {
    streams: Arc<Mutex<HashMap<u32, UserStream>>>, // Map SSRC -> Stream
}

impl VoiceReceiver {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SongbirdEventHandler for VoiceReceiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::VoicePacket(dict) = ctx {
            if let Some(rtp) = dict.rtp.as_ref() {
                // RTP packet
                let ssrc = rtp.ssrc;
                let payload = &rtp.payload; // This is the Opus payload
                
                let mut streams = self.streams.lock().unwrap();
                let stream = streams.entry(ssrc).or_insert_with(|| {
                    tracing::info!("New voice stream: SSRC {}", ssrc);
                    UserStream::new(VOSK_MODEL.clone())
                });
                
                stream.process_packet(payload);
            }
        }
        None
    }
}
