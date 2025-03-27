#![warn(clippy::all, clippy::pedantic)]
#![allow(
    // Allow truncation when casting from usize to i32 since particle coordinates are always small enough to fit in i32
    clippy::cast_possible_truncation,
    // Allow sign loss when going from signed to unsigned types since we validate values are non-negative before casting
    clippy::cast_sign_loss,
    // Allow precision loss when casting between numeric types since exact precision isn't critical for particle effects
    clippy::cast_precision_loss,
    // Allow potential wrapping when casting between types of same size as we validate values are in range
    clippy::cast_possible_wrap,
    // Allow defining constants after statements in functions as it's clearer to define them near where they're used
    clippy::items_after_statements
)]

use bevy_ecs::prelude::*;
use log::{debug, trace};
use ratatui::style::Color;

use crate::components::{Particle, Position, Tetromino};
use crate::screenshake;

pub fn spawn_lock_particles(world: &mut World, position: Position, tetromino: &Tetromino) {
    // Clear any existing coyote time particles first
    clear_coyote_time_particles(world);

    debug!(
        "Spawning lock particles at position ({}, {})",
        position.x, position.y
    );

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();
    let color = tetromino.tetromino_type.get_color();

    const PARTICLES_PER_BLOCK: usize = 8;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity (with upward bias for collision effect)
            let vx = (fastrand::f32() - 0.5) * 4.0;
            let vy = (fastrand::f32() - 0.7) * 4.0; // Bias upward

            spawn_particle(
                world,
                block_pos,
                (vx, vy),
                color,
                fastrand::f32() * 0.8 + 0.2, // lifetime: 0.2 to 1.0 seconds
                fastrand::f32() * 0.8 + 0.2,
            ); // size: 0.2 to 1.0
        }
    }

    // Trigger screen shake effect
    screenshake::trigger_screen_shake(world, 0.8, 0.3);
}

pub fn spawn_rotation_particles(world: &mut World, position: Position, tetromino: &Tetromino) {
    trace!("Spawning rotation particles");

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();
    let color = tetromino.tetromino_type.get_color();

    const PARTICLES_PER_BLOCK: usize = 3;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity in all directions
            let vx = (fastrand::f32() - 0.5) * 2.0;
            let vy = (fastrand::f32() - 0.5) * 2.0;

            spawn_particle(
                world,
                block_pos,
                (vx, vy),
                color,
                fastrand::f32() * 0.4 + 0.1, // lifetime: 0.1 to 0.5 seconds
                fastrand::f32() * 0.6 + 0.2,
            ); // size: 0.2 to 0.8
        }
    }
}

pub fn spawn_coyote_time_particles(world: &mut World, position: Position, tetromino: &Tetromino) {
    // Only spawn particles if we haven't already spawned them for this position
    let already_has_particles = world
        .query::<&Particle>()
        .iter(world)
        .any(|p| p.position.x == position.x && p.position.y == position.y);

    if already_has_particles {
        return;
    }

    debug!(
        "Spawning coyote time particles at position ({}, {})",
        position.x, position.y
    );

    // Get tetromino blocks to spawn particles at each block position
    let blocks = tetromino.get_blocks();

    const PARTICLES_PER_BLOCK: usize = 12;

    // Create particles for each block of the tetromino
    for (block_x, block_y) in blocks {
        let block_pos = Position {
            x: position.x + block_x,
            y: position.y + block_y,
        };

        // Create multiple particles per block
        for _ in 0..PARTICLES_PER_BLOCK {
            // Random velocity with a more scattered pattern
            let vx = (fastrand::f32() - 0.5) * 5.0;
            let vy = (fastrand::f32() - 0.5) * 5.0;

            spawn_particle(
                world,
                block_pos,
                (vx, vy),
                Color::White,
                fastrand::f32() * 0.5 + 0.2, // lifetime: 0.2 to 0.7 seconds
                fastrand::f32() * 0.9 + 0.3,
            ); // size: 0.3 to 1.2
        }
    }

    // Add screen shake with less intensity
    screenshake::trigger_screen_shake(world, 1.0, 0.1);
}

pub fn spawn_perfect_clear_particles(world: &mut World, board_width: usize, board_height: usize) {
    // Create a burst of particles across the entire bottom of the board
    for x in 0..board_width {
        let particle_pos = Position {
            x: i32::try_from(x).unwrap_or(0),
            y: i32::try_from(board_height).unwrap_or(i32::MAX) - 1,
        };

        // Spawn extra particles for impressive clears
        for _ in 0..20 {
            let vx = (fastrand::f32() - 0.5) * 10.0;
            let vy = (fastrand::f32() - 0.8) * 10.0; // Bias upward

            spawn_particle(
                world,
                particle_pos,
                (vx, vy),
                Color::Yellow,
                fastrand::f32() * 1.2 + 0.5, // lifetime: 0.5 to 1.7 seconds
                fastrand::f32() * 1.5 + 0.5,
            ); // size: 0.5 to 2.0
        }
    }

    // Add intense screen shake for perfect clears
    screenshake::trigger_screen_shake(world, 5.0, 0.8);
}

/// Spawns particles for line clear effect
pub fn spawn_line_clear_particles(world: &mut World, board_width: usize, lines: &[usize]) {
    debug!("Spawning line clear particles for {} lines", lines.len());

    // Create particles along each cleared line
    for &y in lines {
        for x in 0..board_width {
            let particle_pos = Position {
                x: i32::try_from(x).unwrap_or(0),
                y: i32::try_from(y).unwrap_or(0),
            };

            // Choose color based on number of lines cleared
            let color = match lines.len() {
                2 => Color::LightBlue,
                3 => Color::LightGreen,
                4 => Color::LightYellow, // Tetris gets bright yellow
                _ => Color::White,       // Single line clears get white
            };

            // Particles per cell depends on number of lines cleared
            let particles_per_cell = 3 + lines.len();

            // Create multiple particles per cell
            for _ in 0..particles_per_cell {
                // Horizontal bias for velocity
                let vx = (fastrand::f32() - 0.5) * 8.0;
                let vy = (fastrand::f32() - 0.5) * 3.0; // Less vertical movement

                spawn_particle(
                    world,
                    particle_pos,
                    (vx, vy),
                    color,
                    fastrand::f32() * 0.7 + 0.3, // lifetime: 0.3 to 1.0 seconds
                    fastrand::f32() * 0.6 + 0.3, // size: 0.3 to 0.9
                );
            }
        }
    }

    // Trigger the specialized line clear screen shake
    screenshake::trigger_line_clear_shake(world, lines.len());
}

pub fn update_particles(world: &mut World, delta_seconds: f32) {
    // First update all particle lifetimes and collect entities to despawn
    let mut entities_to_despawn = Vec::new();

    // Update lifetimes and collect expired particles
    for (entity, mut particle) in world.query::<(Entity, &mut Particle)>().iter_mut(world) {
        // Reduce lifetime
        particle.lifetime -= delta_seconds;

        if particle.lifetime <= 0.0 {
            entities_to_despawn.push(entity);
        }
    }

    // Remove expired particles
    for entity in entities_to_despawn {
        world.despawn(entity);
    }

    // Update remaining particles
    for (_, mut particle) in world.query::<(Entity, &mut Particle)>().iter_mut(world) {
        // Update position based on velocity
        #[allow(clippy::cast_precision_loss)]
        let x_float = particle.position.x as f32;
        let new_x = x_float + particle.velocity.0 * delta_seconds;
        particle.position.x = new_x as i32;

        #[allow(clippy::cast_precision_loss)]
        let y_float = particle.position.y as f32;
        let new_y = y_float + particle.velocity.1 * delta_seconds;
        particle.position.y = new_y as i32;

        // Slow down velocity over time (friction)
        particle.velocity.0 *= 0.95;
        particle.velocity.1 *= 0.95;

        // Add some gravity
        particle.velocity.1 += delta_seconds * 1.0;

        // Gradually decrease size as lifetime decreases, for smoother fade-out
        // Store initial size in an arbitrary safe range (most particles are < 1.0)
        let max_lifetime = 10.0; // Reasonable maximum lifetime estimate
        let fade_factor = (particle.lifetime / max_lifetime).min(1.0);

        // Apply non-linear fade for smoother transition
        // Use a quadratic curve for more graceful fade-out
        particle.size = particle.size * (0.6 + 0.4 * fade_factor * fade_factor);
    }

    // Update screen shake using the dedicated module
    screenshake::update_screen_shake(world, delta_seconds);
}

// Helper function to spawn a single particle
fn spawn_particle(
    world: &mut World,
    position: Position,
    velocity: (f32, f32),
    color: Color,
    lifetime: f32,
    size: f32,
) {
    // Distribute particle sizes more evenly across the whole range
    // Make sure to preserve the base size while adding more variance
    let adjusted_size = size * (0.7 + fastrand::f32() * 0.6);

    world.spawn(Particle {
        position,
        velocity,
        color,
        lifetime,
        size: adjusted_size,
    });
}

// Helper function to clear coyote time particles
fn clear_coyote_time_particles(world: &mut World) {
    let particles_to_remove: Vec<Entity> = world
        .query::<(Entity, &Particle)>()
        .iter(world)
        .filter(|(_, p)| p.color == Color::White) // Coyote time particles are white
        .map(|(e, _)| e)
        .collect();

    for entity in particles_to_remove {
        world.despawn(entity);
    }
}

pub fn create_random_menu_particle() -> Particle {
    // Random position along the top or sides of the screen
    let (x, y, vx, vy) = match fastrand::u8(0..3) {
        0 => {
            // Top - fall down
            let x = fastrand::f32() * 100.0;
            let y = -5.0;
            let vx = (fastrand::f32() - 0.5) * 0.5; // Minimal horizontal movement
            let vy = fastrand::f32() * 0.5 + 0.1; // Falling down
            (x, y, vx, vy)
        }
        1 => {
            // Left side - move right
            let x = -5.0;
            let y = fastrand::f32() * 30.0;
            let vx = fastrand::f32() * 0.5 + 0.1; // Moving right
            let vy = (fastrand::f32() - 0.5) * 0.5; // Minimal vertical movement
            (x, y, vx, vy)
        }
        _ => {
            // Right side - move left
            let x = 100.0;
            let y = fastrand::f32() * 30.0;
            let vx = -fastrand::f32() * 0.5 - 0.1; // Moving left
            let vy = (fastrand::f32() - 0.5) * 0.5; // Minimal vertical movement
            (x, y, vx, vy)
        }
    };

    // Choose a random color
    let color = match fastrand::u8(0..6) {
        0 => Color::Red,
        1 => Color::Green,
        2 => Color::Yellow,
        3 => Color::Blue,
        4 => Color::Magenta,
        _ => Color::Cyan,
    };

    // Create the particle
    Particle {
        position: Position {
            x: x as i32,
            y: y as i32,
        },
        velocity: (vx, vy),
        lifetime: fastrand::f32() * 10.0 + 5.0, // Long-lived particles for menu
        color,
        size: fastrand::f32() * 0.5 + 0.5, // Medium sized
    }
}
