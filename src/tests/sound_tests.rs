#![cfg(test)]

use bevy_ecs::prelude::*;
use std::time::Duration;

use crate::sound::{AudioState, SoundEffect};

#[test]
fn test_audiostate_creation() {
    // Test that AudioState can be created
    let audio_state = AudioState::new();

    // Check default values
    assert_eq!(audio_state.get_volume(), 0.5);
    assert_eq!(audio_state.is_music_enabled(), true);
}

#[test]
fn test_volume_adjustment() {
    // Test volume adjustment functionality
    let mut audio_state = AudioState::new();

    // Test setting volume to normal values
    audio_state.set_volume(0.8);
    assert_eq!(audio_state.get_volume(), 0.8);

    // Test setting volume to higher than 1.0 (should clamp to 1.0)
    audio_state.set_volume(1.5);
    assert_eq!(audio_state.get_volume(), 1.0);

    // Test setting volume to lower than 0.0 (should clamp to 0.0)
    audio_state.set_volume(-0.5);
    assert_eq!(audio_state.get_volume(), 0.0);
}

#[test]
fn test_audiostate_as_resource() {
    // Create a new world
    let mut world = World::new();

    // Add AudioState as a resource
    world.insert_resource(AudioState::new());

    // Check that we can retrieve the resource
    let audio_state = world.resource::<AudioState>();
    assert_eq!(audio_state.get_volume(), 0.5);

    // Modify the resource through a system
    fn modify_volume_system(mut audio_state: ResMut<AudioState>) {
        audio_state.set_volume(0.7);
    }

    // Run the system
    let mut schedule = Schedule::default();
    schedule.add_systems(modify_volume_system);
    schedule.run(&mut world);

    // Check that the modification was applied
    let audio_state = world.resource::<AudioState>();
    assert_eq!(audio_state.get_volume(), 0.7);
}

#[test]
fn test_play_sound() {
    // Create a new world
    let mut world = World::new();

    // Add AudioState as a resource
    world.insert_resource(AudioState::new());

    // Test playing different sound effects (this mostly tests that the API doesn't panic)
    let audio_state = world.resource::<AudioState>();

    // Test each sound effect
    audio_state.play_sound(SoundEffect::Move);
    audio_state.play_sound(SoundEffect::Rotate);
    audio_state.play_sound(SoundEffect::SoftDrop);
    audio_state.play_sound(SoundEffect::HardDrop);
    audio_state.play_sound(SoundEffect::LineClear);
    audio_state.play_sound(SoundEffect::Tetris);
    audio_state.play_sound(SoundEffect::TSpin);
    audio_state.play_sound(SoundEffect::GameOver);
    audio_state.play_sound(SoundEffect::LevelUp);
    audio_state.play_sound(SoundEffect::PerfectClear);

    // Also test the Block* variants
    audio_state.play_sound(SoundEffect::BlockMove);
    audio_state.play_sound(SoundEffect::BlockRotate);
    audio_state.play_sound(SoundEffect::BlockPlace);

    // Small delay to allow thread communication
    std::thread::sleep(Duration::from_millis(10));
}

#[test]
fn test_toggle_music() {
    // Create a new AudioState
    let mut audio_state = AudioState::new();

    // Test toggling music on/off
    let initial_state = audio_state.is_music_enabled();
    audio_state.toggle_music();
    assert_eq!(audio_state.is_music_enabled(), !initial_state);

    // Toggle again to return to initial state
    audio_state.toggle_music();
    assert_eq!(audio_state.is_music_enabled(), initial_state);
}

#[test]
fn test_sound_generation() {
    // Test that our sound sample generation doesn't panic
    for effect in [
        SoundEffect::Move,
        SoundEffect::Rotate,
        SoundEffect::SoftDrop,
        SoundEffect::HardDrop,
        SoundEffect::LineClear,
        SoundEffect::Tetris,
        SoundEffect::TSpin,
        SoundEffect::GameOver,
        SoundEffect::LevelUp,
        SoundEffect::PerfectClear,
        SoundEffect::BlockMove,
        SoundEffect::BlockRotate,
        SoundEffect::BlockPlace,
    ] {
        // Test at different times
        for t in [0.0, 0.1, 0.5, 1.0, 2.0] {
            let (_left, _right) = crate::sound::generate_sound_sample(effect, t);
            // Just checking that it doesn't panic
        }
    }
}

#[test]
fn test_default_audistate() {
    // Test that AudioState implements Default correctly
    let audio_state = AudioState::default();
    assert_eq!(audio_state.get_volume(), 0.5);
    assert_eq!(audio_state.is_music_enabled(), true);
}
