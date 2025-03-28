use bevy_ecs::prelude::*;
use std::time::{Duration, Instant};

use crate::{
    components::{Position, TetrominoType},
    sound::{AudioState, SoundEffect},
    tower_defense::{Enemy, PathSegment, Tower, TowerDefensePath, TowerDefenseState},
};

/// Process enemy movement along path
pub fn process_enemy_movement(world: &mut World) {
    // First collect path segments and create a mapping of segments to positions
    let mut segment_positions = std::collections::HashMap::new();
    let mut segment_entities = Vec::new();

    {
        let mut segments_query = world.query::<(Entity, &PathSegment)>();
        for (entity, segment) in segments_query.iter(world) {
            segment_positions.insert(entity, segment.position);
            segment_entities.push(entity);
        }
    }

    // Now collect the enemies that need processing
    let mut enemies_to_process = Vec::new();
    let mut end_segments = Vec::new();

    // First, collect all end segments
    {
        let mut segments_query = world.query::<(Entity, &PathSegment)>();
        for (entity, segment) in segments_query.iter(world) {
            if segment.next_segment.is_none() {
                end_segments.push(entity);
            }
        }
    }

    // Now collect enemies that are on end segments
    {
        let mut enemy_query = world.query::<(Entity, &Enemy, &mut Position)>();
        for (entity, enemy, _) in enemy_query.iter_mut(world) {
            if let Some(_segment_pos) = segment_positions.get(&enemy.current_segment) {
                if end_segments.contains(&enemy.current_segment) {
                    enemies_to_process.push(entity);
                }
            }
        }
    }

    // Process enemies that reached the end
    for entity in enemies_to_process {
        // Despawn the entity
        world.despawn(entity);

        // Update lives
        if let Some(mut state) = world.get_resource_mut::<TowerDefenseState>() {
            state.lives = state.lives.saturating_sub(1);
        }
    }
}

/// Process tower attacks
pub fn process_tower_attacks(world: &mut World) {
    // First collect all tower data
    let mut towers = Vec::new();
    {
        let mut tower_query = world.query::<(Entity, &Tower, &Position)>();
        for (entity, tower, pos) in tower_query.iter(world) {
            towers.push((entity, tower.clone(), *pos));
        }
    }

    // Now process each tower separately
    for (_tower_entity, tower, tower_pos) in towers {
        // Find enemies in range
        let mut enemies_in_range = Vec::new();
        {
            let mut enemy_query = world.query::<(Entity, &Enemy, &Position)>();
            for (enemy_entity, enemy, enemy_pos) in enemy_query.iter(world) {
                // Check if in range
                let dx = (tower_pos.x - enemy_pos.x) as f32;
                let dy = (tower_pos.y - enemy_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= tower.range {
                    enemies_in_range.push((enemy_entity, enemy.health, enemy.value));
                }
            }
        }

        // Process enemies that are in range
        for (enemy_entity, health, value) in enemies_in_range {
            if health <= tower.damage as u32 {
                // Enemy defeated - update currency first
                if let Some(mut state) = world.get_resource_mut::<TowerDefenseState>() {
                    state.currency += value;
                }
                // Then despawn
                world.despawn(enemy_entity);
            } else {
                // Just reduce health
                let mut enemy_query = world.query::<&mut Enemy>();
                if let Ok(mut enemy) = enemy_query.get_mut(world, enemy_entity) {
                    enemy.health = enemy.health.saturating_sub(tower.damage as u32);
                }
            }
        }
    }
}

/// Handle wave completion check and currency bonuses
pub fn check_wave_completion(world: &mut World) -> bool {
    // First get the enemy count
    let enemy_count = {
        let mut enemy_query = world.query::<&Enemy>();
        enemy_query.iter(world).count()
    };

    let mut wave_completed = false;
    let mut bonus_currency = 0;

    // Now check if the wave is completed
    {
        if let Some(mut state) = world.get_resource_mut::<TowerDefenseState>() {
            if state.enemies_spawned >= state.enemies_to_spawn && enemy_count == 0 {
                // Wave is completed!
                wave_completed = true;
                state.wave_in_progress = false;
                state.wave_completed = true;
                bonus_currency = 50 + (state.wave * 10); // Bonus currency
                state.wave += 1;
            }
        }
    }

    // If wave completed, award bonus currency
    if wave_completed {
        if let Some(mut state) = world.get_resource_mut::<TowerDefenseState>() {
            state.currency += bonus_currency;

            // Play sound effect
            drop(state);
            if let Some(audio_state) = world.get_resource_mut::<AudioState>() {
                if audio_state.is_sound_enabled() {
                    audio_state.play_sound(SoundEffect::Tetris);
                }
            }
        }
    }

    wave_completed
}

/// Spawn new enemies based on current wave state
pub fn spawn_enemies(world: &mut World) {
    // First check if we need to spawn and get required data
    let should_spawn;
    let current_wave;
    let is_first_spawn_of_wave;

    {
        if let Some(mut state) = world.get_resource_mut::<TowerDefenseState>() {
            // Check if this is the first enemy of the wave
            is_first_spawn_of_wave = state.wave_in_progress && state.enemies_spawned == 0;

            // Spawn if:
            // 1. Wave is in progress
            // 2. We haven't spawned all enemies yet
            // 3. Either it's the first enemy (spawn immediately) OR enough time has passed since last spawn
            should_spawn = state.wave_in_progress
                && state.enemies_spawned < state.enemies_to_spawn
                && (is_first_spawn_of_wave
                    || state.next_enemy_spawn.elapsed() > Duration::from_secs(1));

            if should_spawn {
                current_wave = state.wave;
                state.enemies_spawned += 1;
                state.next_enemy_spawn = Instant::now();

                // Log for debugging
                println!(
                    "Spawning enemy {}/{} in wave {}",
                    state.enemies_spawned, state.enemies_to_spawn, state.wave
                );
            } else {
                return; // Early return if no spawning is needed
            }
        } else {
            return; // Early return if no state resource found
        }
    }

    // Get path data separately
    let start_entity;
    let start_position;

    {
        // Get start segment entity from path resource
        if let Some(path) = world.get_resource::<TowerDefensePath>() {
            start_entity = path.start;
        } else {
            println!("No path resource found!");
            return; // Early return if no path resource found
        }
    }

    // Get position from segment entity
    {
        let mut segments_query = world.query::<&PathSegment>();
        if let Ok(segment) = segments_query.get(world, start_entity) {
            start_position = segment.position;
        } else {
            println!("Start segment not found!");
            return; // Early return if segment not found
        }
    }

    // Create enemy of random type
    let enemy_type = TetrominoType::random();
    let is_armored = current_wave > 2 && fastrand::bool();
    let is_boss = current_wave > 3 && fastrand::bool() && fastrand::u8(0..100) < 10;

    let health = if is_boss {
        25 + current_wave as u32
    } else if is_armored {
        10 + (current_wave as u32 / 2)
    } else {
        5 + (current_wave as u32 / 3)
    };

    let enemy = Enemy {
        tetromino_type: enemy_type,
        health,
        max_health: health,
        speed: 0.5 + (current_wave as f32 * 0.1),
        current_segment: start_entity,
        progress: 0.0,
        value: if is_boss {
            50
        } else if is_armored {
            20
        } else {
            10
        },
        is_armored,
        is_boss,
    };

    println!(
        "Spawning {:?} enemy at position ({}, {})",
        enemy_type, start_position.x, start_position.y
    );

    // Spawn at the start position
    let mut commands = world.commands();
    commands.spawn((
        enemy,
        Position {
            x: start_position.x,
            y: start_position.y,
        },
    ));

    // Play a sound when enemy spawns
    if let Some(audio_state) = world.get_resource_mut::<AudioState>() {
        if audio_state.is_sound_enabled() {
            audio_state.play_sound(SoundEffect::BlockMove);
        }
    }
}
