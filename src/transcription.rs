use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vosk::{Model, Recognizer};

const TARGET_SAMPLE_RATE: f32 = 16000.0;
const BUFFER_SIZE: usize = 1600; // 100ms at 16kHz
const SILENCE_TIMEOUT: Duration = Duration::from_millis(1500); // 1.5 seconds of silence

struct UserAudioState {
    recognizer: Recognizer,
    is_speaking: bool,
    accumulated_text: String,
    audio_buffer: Vec<i16>,
    last_audio_time: Option<Instant>,
}

#[derive(Clone)]
pub struct Receiver {
    model: Arc<Model>,
    audio_states: Arc<Mutex<HashMap<u32, UserAudioState>>>,
}

impl Receiver {
    pub fn new(model: Arc<Model>) -> Self {
        Self {
            model,
            audio_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_or_create_state(&self, ssrc: u32) -> Result<(), String> {
        let mut states = self.audio_states.lock().unwrap();

        if !states.contains_key(&ssrc) {
            let recognizer = Recognizer::new(&self.model, TARGET_SAMPLE_RATE).ok_or(format!(
                "Failed to create Vosk recognizer for SSRC {}",
                ssrc
            ))?;

            states.insert(
                ssrc,
                UserAudioState {
                    recognizer,
                    is_speaking: false,
                    accumulated_text: String::new(),
                    audio_buffer: Vec::with_capacity(BUFFER_SIZE),
                    last_audio_time: None,
                },
            );
        }

        Ok(())
    }

    fn stereo_to_mono(stereo: &[i16]) -> Vec<i16> {
        stereo
            .chunks_exact(2)
            .map(|chunk| ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16)
            .collect()
    }

    fn resample_48k_to_16k(samples: &[i16]) -> Vec<i16> {
        samples.iter().step_by(3).copied().collect()
    }

    fn process_audio(&self, ssrc: u32, audio_data: &[i16]) -> Result<(), String> {
        let mut states = self.audio_states.lock().unwrap();

        if let Some(state) = states.get_mut(&ssrc) {
            // Update last audio time
            state.last_audio_time = Some(Instant::now());
            
            // Audio from VoiceTick is already 48kHz stereo PCM
            let mono = Self::stereo_to_mono(audio_data);
            let resampled = Self::resample_48k_to_16k(&mono);
            println!("[DEBUG] Audio processing - SSRC: {}, stereo: {} samples, mono: {} samples, resampled: {} samples", 
                     ssrc, audio_data.len(), mono.len(), resampled.len());

            // Add resampled audio to buffer
            state.audio_buffer.extend_from_slice(&resampled);
            println!(
                "[DEBUG] Buffer size for SSRC {}: {} samples",
                ssrc,
                state.audio_buffer.len()
            );

            // Process buffer when we have enough samples
            if state.audio_buffer.len() >= BUFFER_SIZE {
                println!(
                    "[DEBUG] Processing buffer for SSRC {} ({} samples)",
                    ssrc,
                    state.audio_buffer.len()
                );

                if state
                    .recognizer
                    .accept_waveform(&state.audio_buffer)
                    .is_ok()
                {
                    let result = state.recognizer.result();
                    if let Some(single) = result.single() {
                        let text = single.text.trim();
                        if !text.is_empty() {
                            println!("[VOSK] Partial result for SSRC {}: {}", ssrc, text);
                            if !state.accumulated_text.is_empty() {
                                state.accumulated_text.push(' ');
                            }
                            state.accumulated_text.push_str(text);
                        }
                    }
                }

                // Clear buffer after processing
                state.audio_buffer.clear();
            }
        }

        Ok(())
    }

    fn finalize_transcription(&self, ssrc: u32) -> Option<String> {
        let mut states = self.audio_states.lock().unwrap();

        if let Some(state) = states.get_mut(&ssrc) {
            // Process any remaining audio in buffer
            if !state.audio_buffer.is_empty() {
                println!(
                    "[DEBUG] Finalizing with remaining buffer: {} samples",
                    state.audio_buffer.len()
                );
                let _ = state.recognizer.accept_waveform(&state.audio_buffer);
                state.audio_buffer.clear();
            }

            let result = state.recognizer.final_result();

            let final_text = if let Some(single) = result.single() {
                single.text.trim().to_string()
            } else {
                String::new()
            };

            let complete_text = if !state.accumulated_text.is_empty() {
                format!("{} {}", state.accumulated_text, final_text)
                    .trim()
                    .to_string()
            } else {
                final_text
            };

            state.accumulated_text.clear();
            state.is_speaking = false;
            state.last_audio_time = None;

            if let Some(new_recognizer) = Recognizer::new(&self.model, TARGET_SAMPLE_RATE) {
                state.recognizer = new_recognizer;
            }

            if !complete_text.is_empty() {
                println!(
                    "[VOSK] Final transcription for SSRC {}: {}",
                    ssrc, complete_text
                );
                return Some(complete_text);
            }
        }

        None
    }

    fn check_silence_timeouts(&self) {
        let now = Instant::now();
        let mut states = self.audio_states.lock().unwrap();
        let mut ssrcs_to_finalize = Vec::new();

        for (ssrc, state) in states.iter() {
            if let Some(last_audio_time) = state.last_audio_time {
                if state.is_speaking && now.duration_since(last_audio_time) > SILENCE_TIMEOUT {
                    println!("[DEBUG] Silence timeout detected for SSRC: {}", ssrc);
                    ssrcs_to_finalize.push(*ssrc);
                }
            }
        }

        drop(states);

        for ssrc in ssrcs_to_finalize {
            if let Some(text) = self.finalize_transcription(ssrc) {
                println!("[DEBUG] Auto-finalized transcription: {}", text);
            }
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::SpeakingStateUpdate(speaking) => {
                let ssrc = speaking.ssrc;
                let is_speaking = !speaking.speaking.is_empty();

                println!(
                    "[DEBUG] Speaking state update - SSRC: {}, is_speaking: {}",
                    ssrc, is_speaking
                );

                {
                    let mut states = self.audio_states.lock().unwrap();
                    if let Some(state) = states.get_mut(&ssrc) {
                        state.is_speaking = is_speaking;
                    }
                }

                if is_speaking {
                    println!("[DEBUG] User started speaking - SSRC: {}", ssrc);
                    if let Err(e) = self.get_or_create_state(ssrc) {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("[DEBUG] User stopped speaking - SSRC: {}", ssrc);
                    if let Some(text) = self.finalize_transcription(ssrc) {
                        println!("[DEBUG] Returned transcription: {}", text);
                    } else {
                        println!("[DEBUG] No transcription to finalize for SSRC: {}", ssrc);
                    }
                }
            }
            EventContext::ClientDisconnect(_) => {}
            EventContext::DriverDisconnect { .. } => {}
            EventContext::VoiceTick(tick) => {
                // Check for silence timeouts first
                self.check_silence_timeouts();
                
                for (ssrc, voice_data) in tick.speaking.iter() {
                    let ssrc = *ssrc;

                    if let Some(decoded) = &voice_data.decoded_voice {
                        println!(
                            "[DEBUG] Processing audio for SSRC: {}, samples: {}",
                            ssrc,
                            decoded.len()
                        );

                        if let Err(e) = self.get_or_create_state(ssrc) {
                            eprintln!("Error: {}", e);
                            continue;
                        }

                        if !decoded.is_empty() {
                            // decoded_voice is already Vec<i16> from Opus decoder
                            if let Err(e) = self.process_audio(ssrc, decoded) {
                                eprintln!("Error processing audio: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }
}
