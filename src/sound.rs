use anyhow::Result;
use bevy_ecs::system::Resource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use crossbeam_channel::{Receiver, Sender, bounded};
use fundsp::hacker32::*;
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
    Quit,
}

// Global audio state
#[derive(Resource)]
pub struct AudioState {
    sender: Option<Sender<AudioCommand>>,
    music_enabled: bool,
    sound_enabled: bool,
    volume: f32,
}

impl AudioState {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(64);

        // Start the audio thread
        thread::spawn(move || {
            if let Err(e) = run_audio_thread(receiver) {
                eprintln!("Audio thread error: {}", e);
            }
        });

        Self {
            sender: Some(sender),
            music_enabled: true,
            sound_enabled: true,
            volume: 0.5, // Default volume of 50%
        }
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

    pub fn toggle_music(&mut self) {
        self.music_enabled = !self.music_enabled;

        // Send music toggle to audio thread
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(AudioCommand::PlayMusic(self.music_enabled));
        }
    }
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

fn run_audio_thread(receiver: Receiver<AudioCommand>) -> Result<()> {
    // Get the default audio device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
    let config = device.default_output_config()?;

    // Simple audio state to track volume and music status
    let mut volume = 0.5f32;
    let mut music_enabled = true;

    // Create a channel for sound effects to be handled by the audio callback
    let (sound_sender, sound_receiver) = bounded::<SoundEffect>(64);
    let (cmd_sender, cmd_receiver) = bounded::<(bool, f32)>(16); // for music state and volume

    // Set up audio stream based on the device's sample format
    let _stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run_audio_stream::<f32>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
        )?,
        cpal::SampleFormat::I16 => run_audio_stream::<i16>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
        )?,
        cpal::SampleFormat::U16 => run_audio_stream::<u16>(
            &device,
            &config.into(),
            sound_receiver,
            cmd_receiver,
            volume,
            music_enabled,
        )?,
        _ => return Err(anyhow::anyhow!("Unsupported audio format")),
    };

    // Keep the thread alive and process commands
    loop {
        match receiver.recv() {
            Ok(command) => match command {
                AudioCommand::PlaySound(effect) => {
                    // Forward sound to the audio stream
                    let _ = sound_sender.try_send(effect);
                }
                AudioCommand::PlayMusic(enabled) => {
                    music_enabled = enabled;
                    let _ = cmd_sender.try_send((enabled, volume));
                }
                AudioCommand::SetVolume(new_volume) => {
                    volume = new_volume;
                    let _ = cmd_sender.try_send((music_enabled, volume));
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
    cmd_receiver: Receiver<(bool, f32)>,
    initial_volume: f32,
    initial_music_enabled: bool,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut music_enabled = initial_music_enabled;
    let mut volume = initial_volume;

    // Track active sound effects - store just the sound type and start time
    let mut active_sounds: Vec<(SoundEffect, f64)> = Vec::new();
    let mut current_time = 0.0;

    // Create audio callback closure
    let mut next_value = move || {
        // Process any audio commands (music toggle, volume)
        while let Ok((new_music_enabled, new_volume)) = cmd_receiver.try_recv() {
            music_enabled = new_music_enabled;
            volume = new_volume;
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
        if music_enabled {
            // Simple low-resource background "music"
            let current_time_f32 = current_time as f32;
            let music_freq = 110.0f32 + (current_time_f32 * 0.1f32).sin() * 10.0f32;
            let music_amp = 0.05f32 * ((current_time_f32 * 0.3f32).sin() * 0.5f32 + 0.5f32);
            let sample = (current_time_f32 * music_freq).sin() * music_amp;
            left += sample;
            right += sample;
        }

        // Increment time (assuming 1/sample_rate seconds per sample)
        current_time += 1.0 / sample_rate;

        // Apply volume control
        left *= volume;
        right *= volume;

        // Apply a limiter to prevent clipping
        if left > 1.0 {
            left = 1.0;
        }
        if left < -1.0 {
            left = -1.0;
        }
        if right > 1.0 {
            right = 1.0;
        }
        if right < -1.0 {
            right = -1.0;
        }

        (left, right)
    };

    // Callback for error handling
    let err_fn = |err| eprintln!("Error in audio stream: {}", err);

    // Create the audio stream
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let sample = next_value();
                let left = T::from_sample(sample.0);
                let right = T::from_sample(sample.1);

                for (channel, sample) in frame.iter_mut().enumerate() {
                    if channel & 1 == 0 {
                        *sample = left;
                    } else {
                        *sample = right;
                    }
                }
            }
        },
        err_fn,
        None,
    )?;

    // Start the stream
    stream.play()?;

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
        SoundEffect::HardDrop | SoundEffect::BlockPlace => {
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

// Create a simple click sound for movement
fn create_move_click() -> Box<dyn AudioUnit> {
    Box::new(sine_hz(220.0) * envelope(|t| if t < 0.05 { 1.0 } else { 0.0 }) * 0.3)
}

// Create a higher pitched click for rotation
fn create_rotate_click() -> Box<dyn AudioUnit> {
    Box::new(sine_hz(440.0) * envelope(|t| if t < 0.05 { 1.0 } else { 0.0 }) * 0.3)
}

// Create a soft drop sound
fn create_soft_drop() -> Box<dyn AudioUnit> {
    Box::new(sine_hz(110.0) * envelope(|t| if t < 0.1 { 1.0 } else { 0.0 }) * 0.3)
}

// Create a hard drop sound
fn create_hard_drop() -> Box<dyn AudioUnit> {
    Box::new(sine_hz(80.0) * envelope(|t| (0.1 - t).max(0.0) * 10.0) * 0.5)
}

// Create a line clear sound
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
fn create_tspin() -> Box<dyn AudioUnit> {
    // Create a sound that goes up and back down
    let sweep = envelope(|t| lerp11(200.0, 600.0, sin_hz(1.0, t))) >> sine();

    let node = sweep * envelope(|t| if t < 0.2 { t * 5.0 } else { (1.0 - t).max(0.0) }) * 0.4;

    Box::new(node)
}

// Create a game over sound
fn create_game_over() -> Box<dyn AudioUnit> {
    // Descending pitch
    let sweep = envelope(|t| lerp11(600.0, 200.0, t)) >> sine();

    let node = sweep * envelope(|t| (2.0 - t).max(0.0) * 0.5) * 0.4;
    Box::new(node)
}

// Create a level up sound
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

// Create a perfect clear sound
fn create_perfect_clear() -> Box<dyn AudioUnit> {
    // Special sound for perfect clear
    let sweep1 = envelope(|t| lerp11(400.0, 800.0, t * 2.0)) >> sine();
    let sweep2 = envelope(|t| lerp11(500.0, 1000.0, t * 2.0)) >> sine();

    let env = envelope(|t| if t < 0.3 { t * 3.0 } else { (1.5 - t).max(0.0) });

    let node = (sweep1 + sweep2) * env * 0.4;
    Box::new(node)
}

// Create a sound effect based on type
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
    }
}

// Create background music
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

fn create_block_move_sound() -> Box<dyn AudioUnit> {
    // Create a short clicking sound when blocks move
    let click = sine_hz(200.0) * lfo(|t| exp(-30.0 * t)) * 0.1;
    // Convert to stereo output with centered panning
    Box::new(click >> pan(0.0))
}

fn create_block_rotate_sound() -> Box<dyn AudioUnit> {
    // Create a swishing sound for rotation
    let duration = 0.1;
    let swish =
        sine_hz(300.0) * lfo(move |t| if t < duration { exp(-10.0 * t) } else { 0.0 }) * 0.15;
    // Convert to stereo output with centered panning
    Box::new(swish >> pan(0.0))
}

fn create_block_place_sound() -> Box<dyn AudioUnit> {
    // Create a thud sound for placing blocks
    // First create noise and sine separately and combine at final stage
    let noise_comp = noise() * lfo(|t| exp(-20.0 * t)) * 0.1;
    let sine_comp = sine_hz(100.0) * lfo(|t| exp(-20.0 * t)) * 0.2;
    let thud = noise_comp + sine_comp;

    // Convert to stereo output with slightly right panning
    Box::new(thud >> pan(0.2))
}

fn create_line_clear_sound() -> Box<dyn AudioUnit> {
    // Create a sweep sound for line clear
    let sweep = (sine_hz(440.0) >> follow(0.01) >> sine() * 0.5) * lfo(|t| exp(-6.0 * t)) * 0.3;
    // Convert to stereo output with slightly left panning
    Box::new(sweep >> pan(-0.2))
}
