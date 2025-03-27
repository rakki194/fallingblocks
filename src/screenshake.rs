#![warn(clippy::all, clippy::pedantic)]
#![allow(
    // Allow truncation when casting from usize to i32 since coordinates are always small enough to fit in i32
    clippy::cast_possible_truncation,
    // Allow sign loss when going from signed to unsigned types since we validate values are non-negative before casting
    clippy::cast_sign_loss,
    // Allow precision loss when casting between numeric types since exact precision isn't critical for visual effects
    clippy::cast_precision_loss,
    // Allow potential wrapping when casting between types of same size as we validate values are in range
    clippy::cast_possible_wrap
)]

use bevy_ecs::prelude::*;
use log::trace;

use crate::components::ScreenShake;

/// Triggers a screen shake effect with the specified intensity and duration
pub fn trigger_screen_shake(world: &mut World, intensity: f32, duration: f32) {
    let mut screen_shake = world.resource_mut::<ScreenShake>();
    screen_shake.intensity = intensity;
    screen_shake.duration = duration;
    screen_shake.is_active = true;
    trace!("Screen shake triggered with intensity {intensity}");
}

/// Triggers a line clear screen shake with horizontal bias
pub fn trigger_line_clear_shake(world: &mut World, lines_cleared: usize) {
    // Scale intensity based on number of lines cleared
    let base_intensity = 1.2;
    let intensity_multiplier = match lines_cleared {
        2 => 1.5,
        3 => 2.0,
        4 => 3.0, // Tetris gets a big shake
        _ => 1.0, // Single line clear gets base multiplier
    };

    let intensity = base_intensity * intensity_multiplier;
    let duration = 0.2 + (lines_cleared as f32 * 0.1); // Longer duration for more lines

    let mut screen_shake = world.resource_mut::<ScreenShake>();
    screen_shake.intensity = intensity;
    screen_shake.duration = duration;
    screen_shake.is_active = true;
    screen_shake.horizontal_bias = true; // Set horizontal bias for line clears

    trace!("Line clear screen shake triggered with intensity {intensity}");
}

/// Updates the screen shake state based on elapsed time
pub fn update_screen_shake(world: &mut World, delta_seconds: f32) {
    let mut screen_shake = world.resource_mut::<ScreenShake>();
    if screen_shake.duration > 0.0 {
        screen_shake.duration -= delta_seconds;

        if screen_shake.duration <= 0.0 {
            // Reset shake when duration expires
            screen_shake.intensity = 0.0;
            screen_shake.current_offset = (0, 0);
            screen_shake.is_active = false;
            screen_shake.horizontal_bias = false;
        } else {
            // Calculate random shake offset based on intensity
            let intensity = screen_shake.intensity * (screen_shake.duration / 0.3); // Fade out
            #[allow(clippy::cast_possible_truncation)]
            let max_offset = (intensity * 2.0) as i16;

            if screen_shake.horizontal_bias {
                // For line clear: more horizontal movement, less vertical
                screen_shake.current_offset = (
                    (fastrand::i16(0..=max_offset) - max_offset / 2),
                    (fastrand::i16(0..=(max_offset / 3)) - max_offset / 6),
                );
            } else {
                // Regular screen shake: equal in both directions
                screen_shake.current_offset = (
                    (fastrand::i16(0..=max_offset) - max_offset / 2),
                    (fastrand::i16(0..=max_offset) - max_offset / 2),
                );
            }
        }
    }
}
