use anyhow::Result;
use bevy_ecs::system::Resource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// Sound effects types that can be played
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Some variants may not be used yet but will be in the future
pub enum SoundEffect {
    BlockMove,
    BlockRotate,
    BlockPlace,
    LineClear,
    GameOver,
    LevelUp,
    Move,
    Rotate,
    SoftDrop,
    HardDrop,
    Tetris,
    TSpin,
    PerfectClear,
}

// Command to control the audio thread
enum AudioCommand {
    PlaySound(SoundEffect),
    PlayMusic(bool), // true to start, false to stop
    SetVolume(f32),  // 0.0 to 1.0
    #[allow(dead_code)]
    Quit,
}

// Active sound effect data
struct ActiveSound {
    effect: SoundEffect,
    start_time: f64,
}

// Audio state shared between threads
struct AudioThreadState {
    active_sounds: Vec<ActiveSound>,
    music_enabled: bool,
    volume: f32,
}

// Global audio state
#[derive(Resource)]
pub struct AudioState {
    sender: Option<Sender<AudioCommand>>,
    music_enabled: bool,
    sound_enabled: bool,
    volume: f32,
    audio_available: Arc<AtomicBool>,
}

impl AudioState {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(64);
        let audio_available = Arc::new(AtomicBool::new(false)); // Start with audio unavailable
        let audio_available_clone = audio_available.clone();

        // Start the audio thread with proper error handling
        std::thread::Builder::new()
            .name("audio_thread".to_string())
            .spawn(move || {
                if let Err(e) = run_audio_thread(receiver, audio_available_clone) {
                    eprintln!("Audio thread error: {}", e);
                    // Thread will exit, but game should continue
                }
            })
            .unwrap_or_else(|e| {
                eprintln!("Failed to spawn audio thread: {}", e);
                std::thread::spawn(|| {}) // Dummy thread to avoid panic
            });

        Self {
            sender: Some(sender),
            music_enabled: true,
            sound_enabled: true,
            volume: 0.5, // Default volume of 50%
            audio_available,
        }
    }

    pub fn is_audio_available(&self) -> bool {
        self.audio_available.load(Ordering::Relaxed)
    }

    pub fn play_sound(&self, effect: SoundEffect) -> bool {
        if self.sound_enabled && self.is_audio_available() {
            if let Some(sender) = &self.sender {
                // Use try_send and ignore errors if channel is disconnected
                let _ = sender.try_send(AudioCommand::PlaySound(effect));
            }
            true
        } else {
            false
        }
    }

    pub fn is_music_enabled(&self) -> bool {
        self.music_enabled
    }

    pub fn is_sound_enabled(&self) -> bool {
        self.sound_enabled
    }

    pub fn toggle_sound(&mut self) {
        self.sound_enabled = !self.sound_enabled;
    }

    pub fn get_volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) {
        // Clamp volume between 0.0 and 1.0
        self.volume = volume.clamp(0.0, 1.0);

        // Send volume change to audio thread only if audio is available
        if self.is_audio_available() {
            if let Some(sender) = &self.sender {
                // Ignore errors if the audio thread is dead
                let _ = sender.try_send(AudioCommand::SetVolume(self.volume));
            }
        }
    }

    pub fn toggle_music(&mut self) {
        self.music_enabled = !self.music_enabled;

        // Send music toggle to audio thread only if audio is available
        if self.is_audio_available() {
            if let Some(sender) = &self.sender {
                // Ignore errors if the audio thread is dead
                let _ = sender.try_send(AudioCommand::PlayMusic(self.music_enabled));
            }
        }
    }
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

fn run_audio_thread(
    receiver: Receiver<AudioCommand>,
    audio_available: Arc<AtomicBool>,
) -> Result<()> {
    // Mark audio as unavailable until proven otherwise
    audio_available.store(false, Ordering::Relaxed);

    // Get host and device with proper error handling
    let host = match get_host_with_timeout(std::time::Duration::from_millis(100)) {
        Some(host) => host,
        None => {
            return Err(anyhow::anyhow!("Timed out while getting audio host"));
        }
    };

    let device = match get_device_with_timeout(&host, std::time::Duration::from_millis(100)) {
        Some(device) => device,
        None => {
            return Err(anyhow::anyhow!("No audio output device found"));
        }
    };

    // Get config with timeout to prevent hanging
    let config = match get_config_with_timeout(&device, std::time::Duration::from_millis(100)) {
        Some(Ok(config)) => config,
        Some(Err(e)) => {
            return Err(anyhow::anyhow!("Failed to get output config: {}", e));
        }
        None => {
            return Err(anyhow::anyhow!("Timed out while getting audio config"));
        }
    };

    // Create shared audio state
    let audio_state = Arc::new(Mutex::new(AudioThreadState {
        active_sounds: Vec::new(),
        music_enabled: false,
        volume: 0.5,
    }));

    // Get sample rate for time tracking
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;

    // Create a stream based on the device's sample format
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => create_audio_stream::<f32>(
            &device,
            &config.into(),
            audio_state.clone(),
            sample_rate,
            channels,
        )?,
        cpal::SampleFormat::I16 => create_audio_stream::<i16>(
            &device,
            &config.into(),
            audio_state.clone(),
            sample_rate,
            channels,
        )?,
        cpal::SampleFormat::U16 => create_audio_stream::<u16>(
            &device,
            &config.into(),
            audio_state.clone(),
            sample_rate,
            channels,
        )?,
        _ => {
            return Err(anyhow::anyhow!("Unsupported audio format"));
        }
    };

    // Try to play the stream
    if let Err(e) = stream.play() {
        return Err(anyhow::anyhow!("Failed to play audio stream: {}", e));
    }

    // Audio is now available
    audio_available.store(true, Ordering::Relaxed);

    // Keep the thread alive and process commands
    loop {
        match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(command) => {
                if let Ok(mut state) = audio_state.lock() {
                    match command {
                        AudioCommand::PlaySound(effect) => {
                            // Add the sound to active sounds
                            state.active_sounds.push(ActiveSound {
                                effect,
                                start_time: 0.0, // The actual time will be calculated relative to current time
                            });
                        }
                        AudioCommand::PlayMusic(enabled) => {
                            state.music_enabled = enabled;
                        }
                        AudioCommand::SetVolume(new_volume) => {
                            state.volume = new_volume;
                        }
                        AudioCommand::Quit => break,
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Timeout is fine, just continue
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                // Channel closed, exit thread
                audio_available.store(false, Ordering::Relaxed);
                break;
            }
        }
    }

    Ok(())
}

// Helper functions to prevent hanging

fn get_host_with_timeout(timeout: std::time::Duration) -> Option<cpal::Host> {
    let (sender, receiver) = bounded(1);

    let _handle = std::thread::spawn(move || {
        let host = cpal::default_host();
        let _ = sender.send(host);
    });

    match receiver.recv_timeout(timeout) {
        Ok(host) => Some(host),
        Err(_) => None,
    }
}

fn get_device_with_timeout(
    _host: &cpal::Host,
    timeout: std::time::Duration,
) -> Option<cpal::Device> {
    let (sender, receiver) = bounded(1);

    // Create a new default host instead of trying to clone the one passed in
    let _handle = std::thread::spawn(move || {
        // This creates a new host in the thread, avoiding lifetime issues
        let thread_host = cpal::default_host();
        if let Some(device) = thread_host.default_output_device() {
            let _ = sender.send(device);
        }
    });

    match receiver.recv_timeout(timeout) {
        Ok(device) => Some(device),
        Err(_) => None,
    }
}

fn get_config_with_timeout(
    device: &cpal::Device,
    timeout: std::time::Duration,
) -> Option<Result<cpal::SupportedStreamConfig, cpal::DefaultStreamConfigError>> {
    let (sender, receiver) = bounded(1);
    let device_clone = device.clone();

    let _handle = std::thread::spawn(move || {
        let config = device_clone.default_output_config();
        let _ = sender.send(config);
    });

    match receiver.recv_timeout(timeout) {
        Ok(config) => Some(config),
        Err(_) => None,
    }
}

fn create_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio_state: Arc<Mutex<AudioThreadState>>,
    sample_rate: f32,
    channels: usize,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    // Global time tracking
    let time = Arc::new(Mutex::new(0.0f32));
    let time_clone = time.clone();

    // Error handling function
    let err_fn = |err| eprintln!("Error in audio stream: {}", err);

    // Build the output stream
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // Lock both the time and audio state
            if let (Ok(mut current_time), Ok(mut state)) = (time_clone.lock(), audio_state.lock()) {
                // Extract values before passing them to write_audio_data to avoid borrow issues
                let music_enabled = state.music_enabled;
                let volume = state.volume;

                write_audio_data(
                    data,
                    channels,
                    sample_rate,
                    &mut current_time,
                    &mut state.active_sounds,
                    music_enabled,
                    volume,
                );
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

// Write audio data to the output buffer
fn write_audio_data<T>(
    output: &mut [T],
    channels: usize,
    sample_rate: f32,
    current_time: &mut f32,
    active_sounds: &mut Vec<ActiveSound>,
    music_enabled: bool,
    volume: f32,
) where
    T: SizedSample + FromSample<f32>,
{
    // Process each audio frame
    for frame in output.chunks_mut(channels) {
        // Generate the basic output
        let mut left = 0.0;
        let mut right = 0.0;

        // Process and remove completed sounds
        active_sounds.retain(|sound| {
            let t = *current_time - sound.start_time as f32;

            // Get the sound sample for this effect
            let sample = generate_sound_sample(sound.effect, t);
            left += sample.0;
            right += sample.1;

            // Keep sound if it's still playing
            match sound.effect {
                SoundEffect::GameOver | SoundEffect::LevelUp | SoundEffect::Tetris => t < 2.0,
                SoundEffect::PerfectClear | SoundEffect::TSpin => t < 1.5,
                SoundEffect::LineClear => t < 0.5,
                _ => t < 0.3,
            }
        });

        // Add background music if enabled
        if music_enabled {
            let music_sample = generate_background_music(*current_time);
            left += music_sample.0;
            right += music_sample.1;
        }

        // Apply volume control
        left *= volume;
        right *= volume;

        // Apply a limiter to prevent clipping
        left = left.clamp(-1.0, 1.0);
        right = right.clamp(-1.0, 1.0);

        // Write the samples to output
        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = T::from_sample(left);
            } else {
                *sample = T::from_sample(right);
            }
        }

        // Increment time (assuming 1/sample_rate seconds per sample)
        *current_time += 1.0 / sample_rate;
    }
}

// Generate a sound sample for a specific effect at time t
pub fn generate_sound_sample(effect: SoundEffect, t: f32) -> (f32, f32) {
    const PI2: f32 = std::f32::consts::PI * 2.0;

    match effect {
        SoundEffect::BlockMove | SoundEffect::Move => {
            // Simple click - short sine wave burst
            let amp = if t < 0.05 { 0.3 } else { 0.0 };
            let sample = (t * 220.0 * PI2).sin() * amp;
            (sample, sample)
        }
        SoundEffect::BlockRotate | SoundEffect::Rotate => {
            // Higher pitched click
            let amp = if t < 0.05 { 0.3 } else { 0.0 };
            let sample = (t * 440.0 * PI2).sin() * amp;
            (sample, sample)
        }
        SoundEffect::SoftDrop => {
            // Soft drop sound
            let amp = if t < 0.1 { 0.3 } else { 0.0 };
            let sample = (t * 110.0 * PI2).sin() * amp;
            (sample, sample)
        }
        SoundEffect::HardDrop | SoundEffect::BlockPlace => {
            // Hard drop thud sound
            let amp = (0.1 - t).max(0.0) * 5.0;
            let sample = (t * 80.0 * PI2).sin() * amp;
            (sample, sample)
        }
        SoundEffect::LineClear => {
            // Rising sweep
            let freq = 300.0 + 500.0 * (t * 5.0).min(1.0);
            let amp = if t < 0.2 {
                1.0
            } else {
                (0.5 - t).max(0.0) * 2.0
            };
            let sample = (t * freq * PI2).sin() * amp * 0.3;
            (sample, sample)
        }
        SoundEffect::Tetris => {
            // 4-note ascending arpeggio
            let (freq, amp) = if t < 0.25 {
                (440.0, 0.4)
            } else if t < 0.5 {
                (554.0, 0.4)
            } else if t < 0.75 {
                (659.0, 0.4)
            } else if t < 1.0 {
                (880.0, 0.4)
            } else {
                (0.0, 0.0)
            };
            let sample = (t * freq * PI2).sin() * amp * 0.4;
            (sample, sample)
        }
        SoundEffect::TSpin => {
            // Warbling sound
            let freq = 200.0 + 400.0 * ((t * PI2).sin() * 0.5 + 0.5);
            let amp = if t < 0.2 { t * 5.0 } else { (1.0 - t).max(0.0) };
            let sample = (t * freq * PI2).sin() * amp * 0.4;
            (sample, sample)
        }
        SoundEffect::GameOver => {
            // Descending pitch
            let freq = 600.0 - 400.0 * t;
            let amp = (2.0 - t).max(0.0) * 0.5;
            let sample = (t * freq * PI2).sin() * amp * 0.4;
            (sample, sample)
        }
        SoundEffect::LevelUp => {
            // Ascending arpeggio
            let (freq, amp) = if t < 0.2 {
                (330.0, 1.0)
            } else if t < 0.4 {
                (392.0, 1.0)
            } else if t < 0.6 {
                (494.0, 1.0)
            } else if t < 1.0 {
                (659.0, 1.0)
            } else {
                (0.0, 0.0)
            };
            let sample = (t * freq * PI2).sin() * amp * 0.4;
            (sample, sample)
        }
        SoundEffect::PerfectClear => {
            // Dual sweep sound
            let freq1 = 400.0 + 400.0 * (t * 2.0).min(1.0);
            let freq2 = 500.0 + 500.0 * (t * 2.0).min(1.0);
            let amp = if t < 0.3 { t * 3.0 } else { (1.5 - t).max(0.0) };
            let sample1 = (t * freq1 * PI2).sin() * 0.5;
            let sample2 = (t * freq2 * PI2).sin() * 0.5;
            let sample = (sample1 + sample2) * amp * 0.4;
            (sample, sample)
        }
    }
}

// Generate background music
fn generate_background_music(t: f32) -> (f32, f32) {
    const PI2: f32 = std::f32::consts::PI * 2.0;

    // Bass note
    let bass_freq = 110.0;
    let bass = (t * bass_freq * PI2).sin() * 0.08;

    // Melody note (choose from pentatonic scale based on time)
    let notes = [220.0, 261.63, 293.66, 349.23, 392.0];
    let idx = ((t * 0.5) % 5.0) as usize;
    let melody_freq = notes[idx];
    let melody = (t * melody_freq * PI2).sin() * 0.1;

    // Combine
    let mix = bass + melody;
    (mix * 0.5, mix * 0.5)
}
