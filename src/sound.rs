use anyhow::Result;
use bevy_ecs::system::Resource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use crossbeam_channel::{Receiver, Sender, bounded};
use fundsp::hacker32::*;
use log::{error, info};
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
    Place,
}

// Music types for different game states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicType {
    None,
    MainMenu,
    GameplayA,
    GameplayB,
    GameplayC,
}

// Command to control the audio thread
enum AudioCommand {
    PlaySound(SoundEffect),
    PlayMusic(bool, MusicType), // (enabled, music_type)
    SetVolume(f32),             // 0.0 to 1.0
    #[allow(dead_code)]
    Quit,
}

// Global audio state
#[derive(Resource)]
pub struct AudioState {
    sender: Option<Sender<AudioCommand>>,
    music_enabled: bool,
    sound_enabled: bool,
    volume: f32,
    current_music: MusicType,
    has_audio_device: bool,
}

impl AudioState {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(64);

        // Check if audio device is available
        let has_audio_device = {
            let host = cpal::default_host();
            host.default_output_device().is_some()
                || host
                    .output_devices()
                    .map_or(false, |devices| devices.count() > 0)
        };

        // Start the audio thread
        thread::spawn(move || {
            if let Err(e) = run_audio_thread(receiver) {
                error!("Audio thread error: {}", e);
            }
        });

        Self {
            sender: Some(sender),
            music_enabled: true,
            sound_enabled: true,
            volume: 0.5, // Default volume of 50%
            current_music: MusicType::None,
            has_audio_device,
        }
    }

    pub fn has_audio_device(&self) -> bool {
        self.has_audio_device
    }

    pub fn play_sound(&self, effect: SoundEffect) -> bool {
        if self.sound_enabled {
            if let Some(sender) = &self.sender {
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

        // Send volume change to audio thread
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(AudioCommand::SetVolume(self.volume));
        }
    }

    pub fn increase_volume(&mut self, amount: f32) {
        let new_volume = (self.volume + amount).clamp(0.0, 1.0);
        self.set_volume(new_volume);
    }

    pub fn decrease_volume(&mut self, amount: f32) {
        let new_volume = (self.volume - amount).clamp(0.0, 1.0);
        self.set_volume(new_volume);
    }

    pub fn toggle_music(&mut self) {
        self.music_enabled = !self.music_enabled;

        // Send music toggle to audio thread
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(AudioCommand::PlayMusic(
                self.music_enabled,
                self.current_music,
            ));
        }
    }

    pub fn play_music(&mut self, music_type: MusicType) {
        self.current_music = music_type;

        // Only send command if music is enabled
        if self.music_enabled {
            if let Some(sender) = &self.sender {
                let _ = sender.try_send(AudioCommand::PlayMusic(true, music_type));
            }
        }
    }

    pub fn get_current_music(&self) -> MusicType {
        self.current_music
    }
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

fn run_audio_thread(receiver: Receiver<AudioCommand>) -> Result<()> {
    // Get all available audio hosts
    let host = cpal::default_host();

    // Try to find any available output device
    let device = match host.default_output_device() {
        Some(device) => device,
        None => {
            // Try to find any available output device from the host
            let devices = match host.output_devices() {
                Ok(devices) => devices.collect::<Vec<_>>(),
                Err(e) => {
                    error!("Failed to get output devices: {}", e);
                    return Err(anyhow::anyhow!("No audio output devices available"));
                }
            };

            if devices.is_empty() {
                error!("No audio output devices found");
                return Err(anyhow::anyhow!("No audio output devices available"));
            }

            // Use the first available device as fallback
            devices.into_iter().next().unwrap()
        }
    };

    // Try to get device name for better error reporting
    let device_name = device
        .name()
        .unwrap_or_else(|_| "Unknown Device".to_string());
    error!("Using audio output device: {}", device_name);

    // Try to get the default configuration, with fallbacks
    let config = match device.default_output_config() {
        Ok(config) => config,
        Err(e) => {
            // Try to get any supported configuration
            error!("Error getting default output config: {}", e);
            match device.supported_output_configs() {
                Ok(mut configs) => match configs.next() {
                    Some(config) => {
                        let sample_rate = config.min_sample_rate().max(config.max_sample_rate());
                        error!("Using fallback config with sample rate: {}", sample_rate.0);
                        config.with_sample_rate(sample_rate)
                    }
                    None => {
                        error!("No supported output configurations found");
                        return Err(anyhow::anyhow!("No supported audio configurations found"));
                    }
                },
                Err(e) => {
                    error!("Failed to get supported output configs: {}", e);
                    return Err(anyhow::anyhow!("Failed to get audio configurations"));
                }
            }
        }
    };

    // Simple audio state to track volume and music status
    let mut volume = 0.5f32;
    let mut music_enabled = true;
    let mut music_type = MusicType::None;

    // Create a channel for sound effects to be handled by the audio callback
    let (sound_sender, sound_receiver) = bounded::<SoundEffect>(64);
    let (cmd_sender, cmd_receiver) = bounded::<(bool, f32, MusicType)>(16); // for music state, volume, and type

    // Set up audio stream based on the device's sample format
    let stream_result = match config.sample_format() {
        cpal::SampleFormat::F32 => run_audio_stream::<f32>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
            music_type,
        ),
        cpal::SampleFormat::I16 => run_audio_stream::<i16>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
            music_type,
        ),
        cpal::SampleFormat::U16 => run_audio_stream::<u16>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
            music_type,
        ),
        _ => Err(anyhow::anyhow!("Unsupported audio format")),
    };

    // Handle stream creation failure gracefully
    let _stream = match stream_result {
        Ok(stream) => Some(stream),
        Err(e) => {
            error!("Failed to create audio stream: {}", e);
            error!("Continuing without audio output");
            None
        }
    };

    // Keep the thread alive and process commands
    loop {
        match receiver.recv() {
            Ok(command) => match command {
                AudioCommand::PlaySound(effect) => {
                    // Forward sound to the audio stream
                    if _stream.is_some() {
                        let _ = sound_sender.try_send(effect);
                    }
                }
                AudioCommand::PlayMusic(enabled, music) => {
                    music_enabled = enabled;
                    music_type = music;
                    if _stream.is_some() {
                        let _ = cmd_sender.try_send((enabled, volume, music));
                    }
                }
                AudioCommand::SetVolume(new_volume) => {
                    volume = new_volume;
                    if _stream.is_some() {
                        let _ = cmd_sender.try_send((music_enabled, volume, music_type));
                    }
                }
                AudioCommand::Quit => break,
            },
            Err(_) => break, // Channel closed
        }
    }

    Ok(())
}

fn run_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sound_receiver: Receiver<SoundEffect>,
    cmd_receiver: Receiver<(bool, f32, MusicType)>,
    initial_volume: f32,
    initial_music_enabled: bool,
    initial_music_type: MusicType,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut music_enabled = initial_music_enabled;
    let mut volume = initial_volume;
    let mut music_type = initial_music_type;

    // Track active sound effects - store just the sound type and start time
    let mut active_sounds: Vec<(SoundEffect, f64)> = Vec::new();
    let mut current_time = 0.0;
    let mut music_time = 0.0;

    // Create audio callback closure
    let mut next_value = move || {
        // Process any audio commands (music toggle, volume)
        while let Ok((new_music_enabled, new_volume, new_music_type)) = cmd_receiver.try_recv() {
            music_enabled = new_music_enabled;
            volume = new_volume;

            // If music type changed, reset music time for clean start
            if music_type != new_music_type {
                music_time = 0.0;
            }

            music_type = new_music_type;
        }

        // Process any new sound effects
        while let Ok(effect) = sound_receiver.try_recv() {
            active_sounds.push((effect, current_time));
        }

        // Generate the basic output
        let mut left = 0.0;
        let mut right = 0.0;

        // Add contribution from active sounds
        let mut sounds_to_remove = Vec::new();
        for (idx, (effect, start_time)) in active_sounds.iter().enumerate() {
            let t = current_time - *start_time;

            // Remove sounds after their expected duration
            let max_duration = 2.0; // Default max duration
            if t > max_duration {
                sounds_to_remove.push(idx);
                continue;
            }

            // Generate the sound sample based on effect type and time
            let sample = generate_sound_sample(*effect, t);
            left += sample.0;
            right += sample.1;
        }

        // Remove expired sounds (in reverse order to maintain correct indices)
        for idx in sounds_to_remove.into_iter().rev() {
            if idx < active_sounds.len() {
                active_sounds.remove(idx);
            }
        }

        // Add background music if enabled
        if music_enabled && music_type != MusicType::None {
            let music_sample = generate_music_sample(music_type, music_time);
            left += music_sample.0;
            right += music_sample.1;
            music_time += 1.0 / sample_rate;

            // Loop music after 30 seconds (typical Tetris theme length)
            if music_time > 30.0 {
                music_time %= 30.0;
            }
        }

        // Advance the global time counter
        current_time += 1.0 / sample_rate;

        // Apply global volume
        left *= volume;
        right *= volume;

        // Return the stereo sample
        (left, right)
    };

    // Create the audio stream with our sample generator
    let err_fn = |err| error!("Audio error: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let sample = next_value();
                let left = T::from_sample(sample.0);
                let right = T::from_sample(sample.1);

                for (channel, output) in frame.iter_mut().enumerate() {
                    if channel & 1 == 0 {
                        *output = left;
                    } else {
                        *output = right;
                    }
                }
            }
        },
        err_fn,
        None,
    )?;

    // Try to play the stream, but handle errors gracefully
    match stream.play() {
        Ok(_) => {
            info!("Audio stream started successfully");
        }
        Err(e) => {
            error!("Failed to play audio stream: {}", e);
        }
    }

    Ok(stream)
}

// Generate a sound sample for a given effect and time
pub fn generate_sound_sample(effect: SoundEffect, t: f64) -> (f32, f32) {
    let t = t as f32; // Convert to f32 for simpler calculations

    // Early exit for sounds that have completed
    if t > 2.0 {
        return (0.0, 0.0);
    }

    match effect {
        SoundEffect::Move | SoundEffect::BlockMove => {
            // Simple click - short sine wave burst
            let amp = if t < 0.05 { 0.3 } else { 0.0 };
            let sample = (t * 220.0 * std::f32::consts::TAU).sin() * amp;
            (sample, sample) // Center panned
        }
        SoundEffect::Rotate | SoundEffect::BlockRotate => {
            // Higher pitched click
            let amp = if t < 0.05 { 0.3 } else { 0.0 };
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * amp;
            (sample, sample) // Center panned
        }
        SoundEffect::SoftDrop => {
            // Soft drop - lower tone
            let amp = if t < 0.1 { 0.3 } else { 0.0 };
            let sample = (t * 110.0 * std::f32::consts::TAU).sin() * amp;
            (sample, sample) // Center panned
        }
        SoundEffect::HardDrop | SoundEffect::BlockPlace | SoundEffect::Place => {
            // Hard drop - thud sound
            let amp = (0.1 - t).max(0.0) * 5.0;
            let noise = fastrand::f32() * 0.1; // Simple noise component
            let tone = (t * 80.0 * std::f32::consts::TAU).sin() * 0.2;
            let sample = (noise + tone) * amp;
            (sample * 0.8, sample * 1.2) // Slightly right panned
        }
        SoundEffect::LineClear => {
            // Line clear - rising sweep
            let freq = 300.0 + 500.0 * (t * 5.0).min(1.0);
            let amp = if t < 0.2 {
                1.0
            } else {
                (0.5 - t).max(0.0) * 2.0
            };
            let sample = (t * freq * std::f32::consts::TAU).sin() * amp * 0.3;
            (sample * 1.2, sample * 0.8) // Slightly left panned
        }
        SoundEffect::Tetris => {
            // Four-note ascending arpeggio
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
            let sample = (t * freq * std::f32::consts::TAU).sin() * amp * 0.4;
            (sample, sample) // Center panned
        }
        SoundEffect::TSpin => {
            // T-spin - warbling sound
            let freq = 200.0 + 400.0 * ((t * 1.0 * std::f32::consts::TAU).sin() * 0.5 + 0.5);
            let amp = if t < 0.2 { t * 5.0 } else { (1.0 - t).max(0.0) };
            let sample = (t * freq * std::f32::consts::TAU).sin() * amp * 0.4;
            (sample, sample) // Center panned
        }
        SoundEffect::GameOver => {
            // Game over - descending pitch
            let freq = 600.0 - 400.0 * t;
            let amp = (2.0 - t).max(0.0) * 0.5;
            let sample = (t * freq * std::f32::consts::TAU).sin() * amp * 0.4;
            (sample, sample) // Center panned
        }
        SoundEffect::LevelUp => {
            // Level up - ascending arpeggio
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
            let sample = (t * freq * std::f32::consts::TAU).sin() * amp * 0.4;
            (sample, sample) // Center panned
        }
        SoundEffect::PerfectClear => {
            // Perfect clear - dual sweep
            let freq1 = 400.0 + 400.0 * (t * 2.0).min(1.0);
            let freq2 = 500.0 + 500.0 * (t * 2.0).min(1.0);
            let amp = if t < 0.3 { t * 3.0 } else { (1.5 - t).max(0.0) };
            let sample1 = (t * freq1 * std::f32::consts::TAU).sin() * 0.5;
            let sample2 = (t * freq2 * std::f32::consts::TAU).sin() * 0.5;
            let sample = (sample1 + sample2) * amp * 0.4;
            (sample, sample) // Center panned
        }
    }
}

// Helper function to process audio and return stereo sample
fn process_audio(unit: &mut Box<dyn AudioUnit>, _t: f64) -> (f32, f32) {
    // Check how many inputs and outputs the unit expects
    let num_inputs = unit.inputs();
    let num_outputs = unit.outputs();

    // Create buffers with EXACTLY the size expected by the AudioUnit
    let input_buffer = vec![0.0f32; num_inputs]; // Exact size required
    let mut output_buffer = vec![0.0f32; num_outputs];

    // Process the audio using tick() with proper buffer arguments
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        unit.tick(&input_buffer, &mut output_buffer)
    })) {
        Ok(_) => {
            // Successfully processed audio
        }
        Err(e) => {
            // Handle panic gracefully
            error!("Panic in audio processing: {:?}", e);
            // Return silence in case of panic
            return (0.0, 0.0);
        }
    }

    // Return a stereo sample, ensuring we don't go out of bounds
    if output_buffer.len() >= 2 {
        (output_buffer[0], output_buffer[1])
    } else if output_buffer.len() == 1 {
        // Duplicate mono output to both channels
        (output_buffer[0], output_buffer[0])
    } else {
        // Fallback in case of empty output
        (0.0, 0.0)
    }
}

// Generate music samples based on the music type and time
pub fn generate_music_sample(music_type: MusicType, t: f64) -> (f32, f32) {
    match music_type {
        MusicType::None => (0.0, 0.0),
        MusicType::MainMenu => {
            let mut unit = create_main_menu_music();
            process_audio(&mut unit, t)
        }
        MusicType::GameplayA => {
            let mut unit = create_gameplay_music_a();
            process_audio(&mut unit, t)
        }
        MusicType::GameplayB => {
            let mut unit = create_gameplay_music_b();
            process_audio(&mut unit, t)
        }
        MusicType::GameplayC => {
            let mut unit = create_gameplay_music_c();
            process_audio(&mut unit, t)
        }
    }
}

// Create main menu music inspired by Game Boy Tetris titles
fn create_main_menu_music() -> Box<dyn AudioUnit> {
    // Main melody sequence inspired by Tetris Type A theme intro
    let melody_sequence = vec![
        (329.63, 0.25),  // E4
        (246.94, 0.25),  // B3
        (261.63, 0.25),  // C4
        (293.66, 0.25),  // D4
        (329.63, 0.125), // E4
        (293.66, 0.125), // D4
        (261.63, 0.25),  // C4
        (246.94, 0.25),  // B3
        (196.00, 0.25),  // G3
        (196.00, 0.25),  // G3
        (261.63, 0.25),  // C4
        (329.63, 0.25),  // E4
        (293.66, 0.25),  // D4
        (246.94, 0.25),  // B3
        (246.94, 0.25),  // B3
        (196.00, 0.25),  // G3
        (196.00, 0.5),   // G3
    ];

    // Create a melody using lfo to generate notes based on time
    let melody = lfo(move |t| {
        let pos = t % 4.0; // Loop every 4 seconds
        let mut idx = 0;
        let mut time_passed = 0.0;

        // Find which note to play based on current time
        while idx < melody_sequence.len() {
            let (_, duration) = melody_sequence[idx];
            if pos < time_passed + duration {
                break;
            }
            time_passed += duration;
            idx += 1;
        }

        // If we've reached the end, return silence
        if idx >= melody_sequence.len() {
            return 0.0;
        }

        // Calculate how far we are into this note (0.0 to 1.0)
        let note_pos = (pos - time_passed) / melody_sequence[idx].1;

        // Get the frequency of the current note
        let freq = melody_sequence[idx].0;

        // Apply envelope to each note
        let envelope = if note_pos < 0.1 {
            // Attack
            note_pos * 10.0
        } else if note_pos > 0.8 {
            // Release
            (1.0 - note_pos) * 5.0
        } else {
            // Sustain
            1.0
        };

        // Generate the sound using a square wave (Game Boy-like sound)
        let pi = std::f32::consts::PI;
        let wave_value = if (freq * pos * 2.0f32 * pi).sin() > 0.0 {
            1.0
        } else {
            -1.0
        };

        wave_value * envelope * 0.2 // Adjust volume
    });

    // Add a simple bass line
    let bass = lfo(move |t| {
        let pos = t % 2.0; // Loop every 2 seconds
        let bass_note = if pos < 1.0 { 98.0 } else { 110.0 };
        let pi = std::f32::consts::PI;

        let wave_value = if (bass_note * pos * 2.0f32 * pi).sin() > 0.0 {
            1.0
        } else {
            -0.8
        };

        wave_value * 0.15 // Lower volume for bass
    });

    // Combine melody and bass
    let music = melody + bass;

    // Convert to stereo with slight stereo spread
    Box::new(music * 0.5 >> split::<U2>() >> (pass() | (pass() >> delay(0.02) * 0.8)))
}

// Create gameplay music inspired by Game Boy Tetris Type A theme
fn create_gameplay_music_a() -> Box<dyn AudioUnit> {
    // Main melody sequence inspired by Tetris Type A theme
    let melody_sequence = vec![
        (659.26, 0.25),  // E5
        (493.88, 0.25),  // B4
        (523.25, 0.25),  // C5
        (587.33, 0.25),  // D5
        (659.26, 0.125), // E5
        (587.33, 0.125), // D5
        (523.25, 0.25),  // C5
        (493.88, 0.25),  // B4
        (392.00, 0.25),  // G4
        (392.00, 0.25),  // G4
        (523.25, 0.25),  // C5
        (659.26, 0.25),  // E5
        (587.33, 0.25),  // D5
        (493.88, 0.25),  // B4
        (392.00, 0.25),  // G4
        (392.00, 0.25),  // G4
        // Repeat with variation
        (587.33, 0.5),  // D5
        (523.25, 0.25), // C5
        (392.00, 0.25), // G4
        (523.25, 0.5),  // C5
        (392.00, 0.5),  // G4
    ];

    // Create a melody using lfo
    let melody = lfo(move |t| {
        let pos = t % 6.0; // Loop every 6 seconds
        let mut idx = 0;
        let mut time_passed = 0.0;

        // Find which note to play based on current time
        while idx < melody_sequence.len() {
            let (_, duration) = melody_sequence[idx];
            if pos < time_passed + duration {
                break;
            }
            time_passed += duration;
            idx += 1;
        }

        // If we've reached the end, return silence
        if idx >= melody_sequence.len() {
            return 0.0;
        }

        // Calculate how far we are into this note (0.0 to 1.0)
        let note_pos = (pos - time_passed) / melody_sequence[idx].1;

        // Get the frequency of the current note
        let freq = melody_sequence[idx].0;

        // Apply envelope to each note
        let envelope = if note_pos < 0.1 {
            // Attack
            note_pos * 10.0
        } else if note_pos > 0.8 {
            // Release
            (1.0 - note_pos) * 5.0
        } else {
            // Sustain
            1.0
        };

        // Generate the sound using a square wave (Game Boy-like sound)
        let pi = std::f32::consts::PI;
        let wave_value = if (freq * pos * 2.0f32 * pi).sin() > 0.0 {
            1.0
        } else {
            -1.0
        };

        wave_value * envelope * 0.15 // Adjust volume
    });

    // Add a bass line
    let bass_pattern = vec![
        (82.41, 0.5),  // E2
        (110.00, 0.5), // A2
        (123.47, 0.5), // B2
        (146.83, 0.5), // D3
    ];

    let bass = lfo(move |t| {
        let pattern_length: f32 = bass_pattern.iter().map(|(_, d)| *d as f32).sum();
        let pos = t as f32 % pattern_length;
        let mut idx = 0;
        let mut time_passed = 0.0f32;

        while idx < bass_pattern.len() {
            let (_, duration) = bass_pattern[idx];
            if pos < time_passed + duration as f32 {
                break;
            }
            time_passed += duration as f32;
            idx += 1;
        }

        if idx >= bass_pattern.len() {
            return 0.0;
        }

        let freq = bass_pattern[idx].0;
        let pi = std::f32::consts::PI;
        let wave_value = (freq * t as f32 * 2.0f32 * pi).sin() * 0.2;

        wave_value
    });

    // Add a percussion element for rhythm
    let percussion = lfo(move |t| {
        let bar_pos = t % 1.0; // One bar per second

        if bar_pos < 0.05 || (bar_pos > 0.5 && bar_pos < 0.55) {
            // Kick drum on beats 1 and 3
            (400.0 * (1.0 - bar_pos * 20.0)).exp() * 0.3 * ((t * 100.0) % 2.0 - 1.0)
        } else if bar_pos > 0.25 && bar_pos < 0.3 || (bar_pos > 0.75 && bar_pos < 0.8) {
            // Snare on beats 2 and 4
            (200.0 * (1.0 - (bar_pos - 0.25) * 20.0)).exp() * 0.2 * ((t * 200.0) % 2.0 - 1.0)
        } else {
            0.0
        }
    });

    // Combine all elements
    let music = melody + bass + percussion;

    // Convert to stereo with slight stereo spread
    Box::new(music * 0.5 >> split::<U2>() >> (pass() | (pass() >> delay(0.01) * 0.9)))
}

// Create gameplay music inspired by Game Boy Tetris Type B theme
fn create_gameplay_music_b() -> Box<dyn AudioUnit> {
    // Melodic sequence inspired by Tetris Type B theme
    let melody_sequence = vec![
        (392.00, 0.25), // G4
        (440.00, 0.25), // A4
        (493.88, 0.25), // B4
        (523.25, 0.25), // C5
        (493.88, 0.25), // B4
        (440.00, 0.25), // A4
        (392.00, 0.5),  // G4
        (392.00, 0.25), // G4
        (440.00, 0.25), // A4
        (493.88, 0.25), // B4
        (523.25, 0.25), // C5
        (493.88, 0.25), // B4
        (440.00, 0.25), // A4
        (392.00, 0.5),  // G4
        (440.00, 0.25), // A4
        (523.25, 0.25), // C5
        (659.26, 0.5),  // E5
        (587.33, 0.25), // D5
        (523.25, 0.25), // C5
        (493.88, 0.5),  // B4
        (440.00, 0.25), // A4
        (523.25, 0.25), // C5
        (659.26, 0.5),  // E5
        (587.33, 0.25), // D5
        (523.25, 0.25), // C5
        (493.88, 0.5),  // B4
    ];

    let melody = lfo(move |t| {
        let pos = t % 8.0; // Loop every 8 seconds
        let mut idx = 0;
        let mut time_passed = 0.0;

        while idx < melody_sequence.len() {
            let (_, duration) = melody_sequence[idx];
            if pos < time_passed + duration {
                break;
            }
            time_passed += duration;
            idx += 1;
        }

        if idx >= melody_sequence.len() {
            return 0.0;
        }

        let note_pos = (pos - time_passed) / melody_sequence[idx].1;
        let freq = melody_sequence[idx].0;

        let envelope = if note_pos < 0.1 {
            note_pos * 10.0
        } else if note_pos > 0.8 {
            (1.0 - note_pos) * 5.0
        } else {
            1.0
        };

        // Mix square and triangle waves for a richer tone
        let pi = std::f32::consts::PI;
        let square = if (freq * pos * 2.0f32 * pi).sin() > 0.0 {
            1.0
        } else {
            -1.0
        };
        let triangle = (freq * pos * 2.0f32 * pi).sin().asin() * 2.0f32 / pi;

        (square * 0.7 + triangle * 0.3) * envelope * 0.15
    });

    // Bass line
    let bass = lfo(move |t| {
        let bar = (t / 2.0).floor() as i32 % 2;
        let bar_pos = t % 2.0;

        let bass_freq = if bar == 0 {
            if bar_pos < 1.0 { 98.0 } else { 110.0 }
        } else {
            if bar_pos < 1.0 { 82.41 } else { 73.42 }
        };

        let pi = std::f32::consts::PI;
        let wave = (bass_freq * t as f32 * 2.0f32 * pi).sin();

        wave * 0.2
    });

    // Percussion for rhythm
    let percussion = lfo(move |t| {
        let bar_pos = t % 0.5; // More frequent pattern

        if bar_pos < 0.05 {
            // Kick drum
            (300.0 * (1.0 - bar_pos * 20.0)).exp() * 0.3 * ((t * 100.0) % 2.0 - 1.0)
        } else if bar_pos > 0.25 && bar_pos < 0.3 {
            // Hi-hat
            (500.0 * (1.0 - (bar_pos - 0.25) * 20.0)).exp() * 0.1 * ((t * 300.0) % 2.0 - 1.0)
        } else {
            0.0
        }
    });

    // Combine all elements
    let music = melody + bass + percussion;

    // Convert to stereo with slight stereo spread
    Box::new(music * 0.5 >> split::<U2>() >> (pass() | (pass() >> delay(0.015) * 0.85)))
}

// Create gameplay music for higher levels - more intense
fn create_gameplay_music_c() -> Box<dyn AudioUnit> {
    // Faster and more intense melody inspired by high-level Tetris gameplay
    let melody_sequence = vec![
        (659.26, 0.125), // E5
        (739.99, 0.125), // F#5
        (783.99, 0.125), // G5
        (880.00, 0.125), // A5
        (783.99, 0.125), // G5
        (739.99, 0.125), // F#5
        (659.26, 0.25),  // E5
        (587.33, 0.125), // D5
        (659.26, 0.125), // E5
        (587.33, 0.125), // D5
        (523.25, 0.125), // C5
        (587.33, 0.125), // D5
        (659.26, 0.125), // E5
        (587.33, 0.25),  // D5
        // Repeat with variation
        (523.25, 0.125), // C5
        (587.33, 0.125), // D5
        (659.26, 0.125), // E5
        (587.33, 0.125), // D5
        (523.25, 0.125), // C5
        (493.88, 0.125), // B4
        (523.25, 0.25),  // C5
    ];

    let melody = lfo(move |t| {
        let pos = t % 4.0; // Faster loop - 4 seconds
        let mut idx = 0;
        let mut time_passed = 0.0;

        while idx < melody_sequence.len() {
            let (_, duration) = melody_sequence[idx];
            if pos < time_passed + duration {
                break;
            }
            time_passed += duration;
            idx += 1;
        }

        if idx >= melody_sequence.len() {
            return 0.0;
        }

        let note_pos = (pos - time_passed) / melody_sequence[idx].1;
        let freq = melody_sequence[idx].0;

        let envelope = if note_pos < 0.1 {
            note_pos * 10.0
        } else if note_pos > 0.8 {
            (1.0 - note_pos) * 5.0
        } else {
            1.0
        };

        // Use a pulse wave with a narrower duty cycle for a sharper sound
        let pulse_width = 0.2; // 20% duty cycle
        let wave_value = if (t * freq) % 1.0 < pulse_width {
            1.0
        } else {
            -1.0
        };

        wave_value * envelope * 0.15
    });

    // More energetic bass line with arpeggios
    let bass = lfo(move |t| {
        let beat = (t * 8.0) as i32 % 16; // 16 subdivisions

        // Arpeggio pattern
        let bass_freq = match beat {
            0 | 1 => 110.0,   // A2
            2 => 146.83,      // D3
            3 => 164.81,      // E3
            4 | 5 => 98.0,    // G2
            6 => 123.47,      // B2
            7 => 146.83,      // D3
            8 | 9 => 82.41,   // E2
            10 => 110.0,      // A2
            11 => 130.81,     // C3
            12 | 13 => 73.42, // D2
            14 => 98.0,       // G2
            15 => 110.0,      // A2
            _ => 110.0,
        };

        // Use a mix of sine and saw waves for a richer bass sound
        let pi = std::f32::consts::PI;
        let sine = (bass_freq * t as f32 * 2.0f32 * pi).sin();
        let saw = 2.0 * ((t as f32 * bass_freq) % 1.0) - 1.0;

        (sine * 0.7 + saw * 0.3) * 0.2
    });

    // More complex percussion
    let percussion = lfo(move |t| {
        let bar_pos = t % 0.25; // Faster rhythm

        if bar_pos < 0.02 {
            // Kick drum
            (400.0 * (1.0 - bar_pos * 50.0)).exp() * 0.3 * ((t * 100.0) % 2.0 - 1.0)
        } else if bar_pos > 0.125 && bar_pos < 0.145 {
            // Snare
            (200.0 * (1.0 - (bar_pos - 0.125) * 50.0)).exp() * 0.2 * ((t * 200.0) % 2.0 - 1.0)
        } else if (bar_pos > 0.0625 && bar_pos < 0.0725) || (bar_pos > 0.1875 && bar_pos < 0.1975) {
            // Hi-hat
            (800.0 * (1.0 - (bar_pos - 0.0625) * 100.0)).exp() * 0.1 * ((t * 300.0) % 2.0 - 1.0)
        } else {
            0.0
        }
    });

    // Add a subtle chord pad for more fullness
    let pad = lfo(move |t| {
        let bar = (t / 4.0).floor() as i32 % 2;

        // Alternate between two chord progressions
        let freqs = if bar == 0 {
            [220.0, 277.18, 329.63, 440.0] // A minor
        } else {
            [196.0, 246.94, 293.66, 392.0] // G major
        };

        // Create a pad sound using sine waves
        let mut pad_sound = 0.0;
        let pi = std::f32::consts::PI;
        for freq in freqs.iter() {
            pad_sound += (freq * t as f32 * 2.0f32 * pi).sin() * 0.04;
        }

        // Apply slow modulation
        pad_sound * (1.0 + 0.2 * (0.5 * t as f32).sin())
    });

    // Combine all elements
    let music = melody + bass + percussion + pad;

    // Convert to stereo with slight stereo spread
    Box::new(music * 0.5 >> split::<U2>() >> (pass() | pass() >> delay(0.01) * 0.5))
}

// Create a simple click or short sound effect with configurable parameters
#[allow(dead_code)]
fn create_simple_sound(frequency: f32, duration: f32, volume: f32) -> Box<dyn AudioUnit> {
    Box::new(sine_hz(frequency) * envelope(move |t| if t < duration { 1.0 } else { 0.0 }) * volume)
}

// Create a simple click sound for movement
#[allow(dead_code)]
fn create_move_click() -> Box<dyn AudioUnit> {
    create_simple_sound(220.0, 0.05, 0.3)
}

// Create a higher pitched click for rotation
#[allow(dead_code)]
fn create_rotate_click() -> Box<dyn AudioUnit> {
    create_simple_sound(440.0, 0.05, 0.3)
}

// Create a soft drop sound
#[allow(dead_code)]
fn create_soft_drop() -> Box<dyn AudioUnit> {
    create_simple_sound(110.0, 0.1, 0.3)
}

// Create a hard drop sound
#[allow(dead_code)]
fn create_hard_drop() -> Box<dyn AudioUnit> {
    Box::new(sine_hz(80.0) * envelope(|t| (0.1 - t).max(0.0) * 10.0) * 0.5)
}

// Create a line clear sound
#[allow(dead_code)]
fn create_line_clear() -> Box<dyn AudioUnit> {
    // Create a rising sweep sound
    let sweep = envelope(|t| lerp11(300.0, 800.0, (t * 5.0).min(1.0))) >> sine();

    let node = sweep
        * envelope(|t| {
            if t < 0.2 {
                1.0
            } else {
                (0.5 - t).max(0.0) * 2.0
            }
        })
        * 0.4;

    Box::new(node)
}

// Create a tetris clear sound
#[allow(dead_code)]
fn create_tetris() -> Box<dyn AudioUnit> {
    // Four-note ascending arpeggio
    let note = |freq, t_start, t_end| {
        let env = envelope(move |t| if t >= t_start && t < t_end { 0.4 } else { 0.0 });
        sine_hz(freq) * env
    };

    let node = (note(440.0, 0.0, 0.25)
        + note(554.0, 0.25, 0.5)
        + note(659.0, 0.5, 0.75)
        + note(880.0, 0.75, 1.0))
        * 0.4;

    Box::new(node)
}

// Create a t-spin sound
#[allow(dead_code)]
fn create_tspin() -> Box<dyn AudioUnit> {
    // Create a sound that goes up and back down
    let sweep = envelope(|t| lerp11(200.0, 600.0, sin_hz(1.0, t))) >> sine();

    let node = sweep * envelope(|t| if t < 0.2 { t * 5.0 } else { (1.0 - t).max(0.0) }) * 0.4;

    Box::new(node)
}

// Create a level up sound
#[allow(dead_code)]
fn create_level_up() -> Box<dyn AudioUnit> {
    // Ascending arpeggio
    let note = |freq, t_start, t_end| {
        let env = envelope(move |t| if t >= t_start && t < t_end { 1.0 } else { 0.0 });
        sine_hz(freq) * env
    };

    let node = (note(330.0, 0.0, 0.2)
        + note(392.0, 0.2, 0.4)
        + note(494.0, 0.4, 0.6)
        + note(659.0, 0.6, 1.0))
        * 0.4;
    Box::new(node)
}

// Helper function to create sweep-based sounds
#[allow(dead_code)]
fn create_sweep_sound(
    start_freq: f32,
    end_freq: f32,
    sweep_time: f32,
    envelope_fn: impl Fn(f32) -> f32 + 'static + Send + Sync + Clone,
    volume: f32,
) -> Box<dyn AudioUnit> {
    // Create a rising sweep sound
    let sweep =
        envelope(move |t| lerp11(start_freq, end_freq, (t * sweep_time).min(1.0))) >> sine();
    let node = sweep * envelope(envelope_fn) * volume;
    Box::new(node)
}

// Create a game over sound
#[allow(dead_code)]
fn create_game_over() -> Box<dyn AudioUnit> {
    // Descending pitch
    let sweep = envelope(|t| lerp11(600.0, 200.0, t)) >> sine();

    let node = sweep * envelope(|t| (2.0 - t).max(0.0) * 0.5) * 0.4;
    Box::new(node)
}

// Create a perfect clear sound
#[allow(dead_code)]
fn create_perfect_clear() -> Box<dyn AudioUnit> {
    // Special sound for perfect clear
    let sweep1 = envelope(|t| lerp11(400.0, 800.0, t * 2.0)) >> sine();
    let sweep2 = envelope(|t| lerp11(500.0, 1000.0, t * 2.0)) >> sine();

    let env = envelope(|t| if t < 0.3 { t * 3.0 } else { (1.5 - t).max(0.0) });

    let node = (sweep1 + sweep2) * env * 0.4;
    Box::new(node)
}

// Create a sound effect based on type
#[allow(dead_code)]
fn create_sound_effect(effect: SoundEffect) -> Box<dyn AudioUnit> {
    match effect {
        SoundEffect::BlockMove => create_block_move_sound(),
        SoundEffect::BlockRotate => create_block_rotate_sound(),
        SoundEffect::BlockPlace => create_block_place_sound(),
        SoundEffect::LineClear => create_line_clear_sound(),
        SoundEffect::GameOver => create_game_over(),
        SoundEffect::LevelUp => create_level_up(),
        SoundEffect::Move => create_move_click(),
        SoundEffect::Rotate => create_rotate_click(),
        SoundEffect::SoftDrop => create_soft_drop(),
        SoundEffect::HardDrop => create_hard_drop(),
        SoundEffect::Tetris => create_tetris(),
        SoundEffect::TSpin => create_tspin(),
        SoundEffect::PerfectClear => create_perfect_clear(),
        SoundEffect::Place => create_block_place_sound(),
    }
}

// Create background music
#[allow(dead_code)]
fn create_background_music() -> Box<dyn AudioUnit> {
    // Create a simple tetris-style background music using fundamental oscillators

    // Bass line - low frequency oscillator
    let bass = sine_hz(110.0) * 0.08;

    // Melody - slightly higher notes that change over time
    let melody = lfo(move |t| {
        // Cycle through a pentatonic scale
        let notes = [220.0, 261.63, 293.66, 349.23, 392.0];
        let idx = ((t * 0.5) % 5.0) as usize;
        notes[idx]
    }) >> sine() * 0.1;

    // Chord pad for harmony - multiple frequencies together
    let chord = sine_hz(220.0) * 0.03 + sine_hz(329.63) * 0.02 + sine_hz(392.0) * 0.02;

    // Rhythmic beeping element
    let rhythm = lfo(move |t| {
        // Create a pulsing rhythm
        if (t * 2.0) % 1.0 < 0.1 { 0.05 } else { 0.0 }
    }) * sine_hz(440.0);

    // Combine all elements and apply volume
    let music = (bass + melody + chord + rhythm) * 0.6;

    // Convert to stereo with center panning
    Box::new(music >> pan(0.0))
}

#[allow(dead_code)]
fn create_block_move_sound() -> Box<dyn AudioUnit> {
    // Create a short clicking sound when blocks move
    let click = sine_hz(200.0) * lfo(|t| exp(-30.0 * t)) * 0.1;
    // Convert to stereo output with centered panning
    Box::new(click >> pan(0.0))
}

#[allow(dead_code)]
fn create_block_rotate_sound() -> Box<dyn AudioUnit> {
    // Create a swishing sound for rotation
    let duration = 0.1;
    let swish =
        sine_hz(300.0) * lfo(move |t| if t < duration { exp(-10.0 * t) } else { 0.0 }) * 0.15;
    // Convert to stereo output with centered panning
    Box::new(swish >> pan(0.0))
}

#[allow(dead_code)]
fn create_block_place_sound() -> Box<dyn AudioUnit> {
    // Create a thud sound for placing blocks
    // First create noise and sine separately and combine at final stage
    let noise_comp = noise() * lfo(|t| exp(-20.0 * t)) * 0.1;
    let sine_comp = sine_hz(100.0) * lfo(|t| exp(-20.0 * t)) * 0.2;
    let thud = noise_comp + sine_comp;

    // Convert to stereo output with slightly right panning
    Box::new(thud >> pan(0.2))
}

#[allow(dead_code)]
fn create_line_clear_sound() -> Box<dyn AudioUnit> {
    // Create a sweep sound for line clear
    let sweep = (sine_hz(440.0) >> follow(0.01) >> sine() * 0.5) * lfo(|t| exp(-6.0 * t)) * 0.3;
    // Convert to stereo output with slightly left panning
    Box::new(sweep >> pan(-0.2))
}
