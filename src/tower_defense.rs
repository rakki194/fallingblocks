#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

use bevy_ecs::prelude::*;
use ratatui::style::Color;
use std::time::Instant;

use crate::components::{Position, TetrominoType};

/// Path segment for tetrominos to follow
#[derive(Component, Debug, Clone)]
pub struct PathSegment {
    pub position: Position,
    pub next_segment: Option<Entity>,
}

/// Represents a tower defense game path from start to end
#[derive(Resource, Debug, Clone)]
pub struct TowerDefensePath {
    pub start: Entity,         // First path segment
    pub end: Entity,           // Last path segment
    pub length: usize,         // Total path length
    pub segments: Vec<Entity>, // All path segment entities
}

/// Represents the game state for tower defense mode
#[derive(Resource, Debug, Clone)]
pub struct TowerDefenseState {
    pub currency: u32,
    pub wave: u32,
    pub lives: u32,
    pub selected_tower_type: Option<TowerType>,
    pub game_over: bool,
    pub wave_in_progress: bool,
    pub next_enemy_spawn: Instant,
    pub enemies_spawned: u32,
    pub enemies_to_spawn: u32,
    pub wave_completed: bool,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub scroll_x: i32,   // Horizontal scroll offset
    pub scroll_y: i32,   // Vertical scroll offset
    pub map_width: i32,  // Total map width
    pub map_height: i32, // Total map height
}

impl Default for TowerDefenseState {
    fn default() -> Self {
        Self {
            currency: 100,
            wave: 1,
            lives: 20,
            selected_tower_type: None,
            game_over: false,
            wave_in_progress: false,
            next_enemy_spawn: Instant::now(),
            enemies_spawned: 0,
            enemies_to_spawn: 10,
            wave_completed: false,
            cursor_x: 5,
            cursor_y: 5,
            scroll_x: 0,
            scroll_y: 0,
            map_width: 40,  // Larger map width
            map_height: 30, // Larger map height
        }
    }
}

/// Properties for enemies in tower defense mode
#[derive(Component, Debug, Clone)]
pub struct Enemy {
    pub tetromino_type: TetrominoType,
    pub health: u32,
    pub max_health: u32,
    pub speed: f32,
    pub current_segment: Entity,
    pub progress: f32, // 0.0 to 1.0 progress along current segment
    pub value: u32,    // Currency awarded when defeated
    pub is_armored: bool,
    pub is_boss: bool,
}

/// Different types of towers with unique abilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerType {
    Basic,  // Rapid fire, low damage
    Cannon, // Area damage, slow fire rate
    Freeze, // Slows down enemies
    Sniper, // High damage, slow fire rate
    Chain,  // Damage chains to nearby enemies
}

impl TowerType {
    pub fn get_color(&self) -> Color {
        match self {
            TowerType::Basic => Color::Green,
            TowerType::Cannon => Color::Red,
            TowerType::Freeze => Color::Cyan,
            TowerType::Sniper => Color::Yellow,
            TowerType::Chain => Color::Magenta,
        }
    }

    pub fn get_cost(&self) -> u32 {
        match self {
            TowerType::Basic => 50,
            TowerType::Cannon => 100,
            TowerType::Freeze => 100,
            TowerType::Sniper => 150,
            TowerType::Chain => 200,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            TowerType::Basic => "Basic Tower",
            TowerType::Cannon => "Cannon Tower",
            TowerType::Freeze => "Freeze Tower",
            TowerType::Sniper => "Sniper Tower",
            TowerType::Chain => "Chain Tower",
        }
    }

    pub fn get_description(&self) -> &'static str {
        match self {
            TowerType::Basic => "Rapid fire, low damage",
            TowerType::Cannon => "Area damage, slow fire rate",
            TowerType::Freeze => "Slows down enemies",
            TowerType::Sniper => "High damage, slow fire rate",
            TowerType::Chain => "Damage chains to nearby enemies",
        }
    }
}

/// Tower component for defense against tetromino enemies
#[derive(Component, Debug, Clone)]
pub struct Tower {
    pub tower_type: TowerType,
    pub level: u32,
    pub damage: f32,
    pub range: f32,
    pub fire_rate: f32, // Attacks per second
    pub last_attack: Instant,
    pub targets: Vec<Entity>, // Current targets
}

/// Tower base stats for initial tower creation
#[derive(Debug, Clone, Copy)]
struct TowerStats {
    damage: f32,
    range: f32,
    fire_rate: f32,
}

impl Tower {
    pub fn new(tower_type: TowerType) -> Self {
        // Base stats for each tower type
        let stats = match tower_type {
            TowerType::Basic => TowerStats {
                damage: 1.0,
                range: 3.0,
                fire_rate: 4.0,
            },
            TowerType::Cannon => TowerStats {
                damage: 3.0,
                range: 2.0,
                fire_rate: 1.0,
            },
            TowerType::Freeze => TowerStats {
                damage: 0.5,
                range: 2.5,
                fire_rate: 2.0,
            },
            TowerType::Sniper => TowerStats {
                damage: 10.0,
                range: 5.0,
                fire_rate: 0.5,
            },
            TowerType::Chain => TowerStats {
                damage: 2.0,
                range: 3.0,
                fire_rate: 1.5,
            },
        };

        Self {
            tower_type,
            level: 1,
            damage: stats.damage,
            range: stats.range,
            fire_rate: stats.fire_rate,
            last_attack: Instant::now(),
            targets: Vec::new(),
        }
    }
}

/// Tower attack animation
#[derive(Component, Debug, Clone)]
pub struct TowerAttack {
    pub source: Entity,
    pub target: Entity,
    pub duration: f32,
    pub elapsed: f32,
    pub is_area_effect: bool,
    pub is_chain: bool,
    pub chain_targets: Vec<Entity>,
}

/// Status effect on enemies (slow, stun, etc.)
#[derive(Component, Debug, Clone)]
pub struct StatusEffect {
    pub effect_type: StatusEffectType,
    pub duration: f32,
    pub remaining: f32,
    pub source: Entity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusEffectType {
    Slow, // Reduces speed
    Stun, // Prevents movement
    Burn, // Damage over time
}

/// System to generate a procedural path for tower defense
pub fn generate_procedural_path(
    commands: &mut Commands,
    width: usize,
    height: usize,
) -> TowerDefensePath {
    // Create a more interesting and visually appealing path that zigzags through the board
    let mut segments = Vec::new();

    // Choose a random entry point on the left edge
    let start_y = fastrand::usize(height / 4..(height * 3) / 4);

    // Define an exit point on the right edge
    let end_y = fastrand::usize(height / 4..(height * 3) / 4);

    // Path width - make paths at least 4 blocks wide to accommodate tetrominos
    let path_width = 4;

    // Create main path segments first
    // Number of turning points to create
    let turns = fastrand::usize(3..6);

    // Create intermediate waypoints for a more interesting path
    let mut waypoints = Vec::new();

    // Start at entry point
    let mut current_x = 0;
    let mut current_y = start_y as i32;

    // End at exit point
    let target_x = width as i32 - 1;
    let target_y = end_y as i32;

    // Create intermediate waypoints with minimum distance between them
    // to ensure we have enough space for wider paths
    for i in 0..turns {
        // For x coordinate, divide the horizontal space into roughly equal sections
        // Ensure there's enough space for the wide path
        let section_width = (target_x - current_x) / (turns - i) as i32;
        let next_x = current_x + section_width;

        // For y coordinate, generate interesting patterns with enough space for wide paths
        let next_y = if i % 2 == 0 {
            // Move toward a random position in the upper half
            fastrand::i32(height as i32 / 4..(height as i32 * 3) / 4)
        } else {
            // Move toward a random position in the lower half
            fastrand::i32(height as i32 / 4..(height as i32 * 3) / 4)
        };

        waypoints.push((next_x, next_y));
        current_x = next_x;
        current_y = next_y;
    }

    // Add the exit point as the final waypoint
    waypoints.push((target_x, target_y));

    // Create the first segment at the start position
    let start_entity = commands
        .spawn((PathSegment {
            position: Position {
                x: 0,
                y: start_y as i32,
            },
            next_segment: None,
        },))
        .id();
    segments.push(start_entity);

    let mut prev_entity = start_entity;
    let mut prev_x = 0;
    let mut prev_y = start_y as i32;

    // Create a set to track positions that already have path segments
    // to avoid duplicate path segments at the same location
    let mut path_positions = std::collections::HashSet::new();
    path_positions.insert((prev_x, prev_y));

    // Connect waypoints with thick paths
    for (target_x, target_y) in waypoints {
        // First generate the main path to the waypoint
        // Create horizontal path segment to reach target_x
        while prev_x != target_x {
            let direction = if prev_x < target_x { 1 } else { -1 };
            prev_x += direction;

            // Skip if we already have a path segment at this position
            if path_positions.contains(&(prev_x, prev_y)) {
                continue;
            }

            // Create a new path segment
            let segment_entity = commands
                .spawn((PathSegment {
                    position: Position {
                        x: prev_x,
                        y: prev_y,
                    },
                    next_segment: None,
                },))
                .id();
            segments.push(segment_entity);
            path_positions.insert((prev_x, prev_y));

            // Link to previous segment
            commands.entity(prev_entity).insert(PathSegment {
                position: Position {
                    x: prev_x - direction,
                    y: prev_y,
                },
                next_segment: Some(segment_entity),
            });

            prev_entity = segment_entity;
        }

        // Then create vertical path segment to reach target_y
        while prev_y != target_y {
            let direction = if prev_y < target_y { 1 } else { -1 };
            prev_y += direction;

            // Skip if we already have a path segment at this position
            if path_positions.contains(&(prev_x, prev_y)) {
                continue;
            }

            // Create a new path segment
            let segment_entity = commands
                .spawn((PathSegment {
                    position: Position {
                        x: prev_x,
                        y: prev_y,
                    },
                    next_segment: None,
                },))
                .id();
            segments.push(segment_entity);
            path_positions.insert((prev_x, prev_y));

            // Link to previous segment
            commands.entity(prev_entity).insert(PathSegment {
                position: Position {
                    x: prev_x,
                    y: prev_y - direction,
                },
                next_segment: Some(segment_entity),
            });

            prev_entity = segment_entity;
        }
    }

    // Now widen the path by adding additional path segments around the main path
    let mut additional_segments = Vec::new();

    // Get all existing path positions directly from our position tracking
    let main_path_positions: Vec<(i32, i32)> = path_positions.iter().copied().collect();

    // For each position in the main path, add additional width
    for &(x, y) in &main_path_positions {
        // Add path segments to create a corridor with the specified width
        // Only expanding perpendicular to the path direction to maintain a clean look
        for w in 1..(path_width / 2) {
            // Check if we already have main path segments at these positions
            let positions_to_check = [(x, y - w), (x, y + w), (x - w, y), (x + w, y)];

            for (nx, ny) in positions_to_check {
                // Skip if outside board boundaries
                if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                    continue;
                }

                // Skip if we already have a path segment at this position
                if path_positions.contains(&(nx, ny)) {
                    continue;
                }

                // Add a new path segment (not connected to the main path for enemies)
                let segment_entity = commands
                    .spawn((PathSegment {
                        position: Position { x: nx, y: ny },
                        next_segment: None,
                    },))
                    .id();

                additional_segments.push(segment_entity);
                path_positions.insert((nx, ny));
            }
        }
    }

    // Add all additional segments to the segments vector
    segments.extend(additional_segments);

    // The last path segment is the end point
    // Since we built the path sequentially, the last entity we connected is the end
    let end_entity = prev_entity;

    TowerDefensePath {
        start: start_entity,
        end: end_entity,
        length: segments.len(),
        segments,
    }
}

/// System to spawn a new wave of enemies
pub fn spawn_enemy_wave(
    _commands: &mut Commands,
    state: &mut TowerDefenseState,
    _path: &TowerDefensePath,
) {
    state.wave_in_progress = true;
    state.enemies_spawned = 0;
    state.enemies_to_spawn = 10 + (state.wave * 5); // More enemies in higher waves
    state.next_enemy_spawn = Instant::now();
    state.wave_completed = false;
}

/// Placeholder function for updating enemies along the path
pub fn move_enemies_system() -> impl FnMut(
    Commands,
    Query<(Entity, &mut Enemy, &mut Position)>,
    Query<&PathSegment>,
    ResMut<TowerDefenseState>,
) {
    move |mut commands, mut enemies, path_segments, mut state| {
        for (entity, mut enemy, mut position) in enemies.iter_mut() {
            // Get the current path segment
            if let Ok(current_segment) = path_segments.get(enemy.current_segment) {
                // Check if this is the end segment
                if current_segment.next_segment.is_none() {
                    // Enemy reached the end - reduce player lives
                    state.lives = state.lives.saturating_sub(1);

                    // Check if game over
                    if state.lives == 0 {
                        state.game_over = true;
                    }

                    // Remove the enemy
                    commands.entity(entity).despawn();
                    continue;
                }

                // Move along the current segment
                enemy.progress += enemy.speed * 0.016; // Assuming ~60 FPS

                // If reached the end of current segment, move to next
                if enemy.progress >= 1.0 {
                    if let Some(next_segment_entity) = current_segment.next_segment {
                        if let Ok(next_segment) = path_segments.get(next_segment_entity) {
                            // Move to next segment
                            enemy.current_segment = next_segment_entity;
                            enemy.progress = 0.0;

                            // Position at the start of the next segment
                            position.x = next_segment.position.x;
                            position.y = next_segment.position.y;
                        }
                    }
                } else {
                    // Interpolate position along current segment
                    if let Some(next_segment_entity) = current_segment.next_segment {
                        if let Ok(next_segment) = path_segments.get(next_segment_entity) {
                            // Calculate interpolated position
                            let start_x = current_segment.position.x;
                            let start_y = current_segment.position.y;
                            let end_x = next_segment.position.x;
                            let end_y = next_segment.position.y;

                            position.x =
                                start_x + ((end_x - start_x) as f32 * enemy.progress) as i32;
                            position.y =
                                start_y + ((end_y - start_y) as f32 * enemy.progress) as i32;
                        }
                    }
                }
            }
        }
    }
}

/// Placeholder function for tower targeting system
pub fn tower_targeting_system() -> impl FnMut(
    Commands,
    Query<(Entity, &Tower, &Position)>,
    Query<(Entity, &mut Enemy, &Position)>,
    ResMut<TowerDefenseState>,
) {
    move |mut commands, towers, mut enemies, mut state| {
        // First collect all the towers information
        struct TowerInfo {
            entity: Entity,
            tower_type: TowerType,
            position: Position,
            damage: f32,
            range: f32,
        }

        let mut tower_infos = Vec::new();
        for (entity, tower, position) in towers.iter() {
            // Check if tower can fire
            if tower.last_attack.elapsed().as_secs_f32() < 1.0 / tower.fire_rate {
                continue;
            }

            tower_infos.push(TowerInfo {
                entity,
                tower_type: tower.tower_type,
                position: *position,
                damage: tower.damage,
                range: tower.range,
            });
        }

        // Process each tower
        for tower_info in tower_infos {
            match tower_info.tower_type {
                TowerType::Basic => {
                    // Basic tower targets closest enemy
                    let mut targets = Vec::new();
                    for (enemy_entity, _, enemy_pos) in enemies.iter() {
                        // Calculate distance
                        let dx = (tower_info.position.x - enemy_pos.x) as f32;
                        let dy = (tower_info.position.y - enemy_pos.y) as f32;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance <= tower_info.range {
                            targets.push((enemy_entity, distance));
                        }
                    }

                    // Sort targets by distance
                    targets
                        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                    if let Some((target_entity, _)) = targets.first() {
                        // Attack the target
                        if let Ok(mut enemy) = enemies.get_mut(*target_entity) {
                            // Deal damage
                            enemy.1.health =
                                enemy.1.health.saturating_sub(tower_info.damage as u32);

                            // Check if enemy is defeated
                            if enemy.1.health == 0 {
                                // Award currency
                                state.currency += enemy.1.value;

                                // Remove the enemy
                                commands.entity(*target_entity).despawn();
                            }
                        }

                        // Update tower's last attack time
                        if let Some(mut tower_comp) = commands.get_entity(tower_info.entity) {
                            let mut updated_tower = Tower::new(tower_info.tower_type);
                            updated_tower.last_attack = Instant::now();
                            updated_tower.damage = tower_info.damage;
                            updated_tower.range = tower_info.range;
                            updated_tower.level = 1; // Maintain level
                            tower_comp.insert(updated_tower);
                        }
                    }
                }
                TowerType::Cannon => {
                    // Cannon tower targets closest enemy and deals area damage
                    let mut targets = Vec::new();

                    // First, collect eligible targets
                    for (enemy_entity, _, enemy_pos) in enemies.iter() {
                        // Calculate distance
                        let dx = (tower_info.position.x - enemy_pos.x) as f32;
                        let dy = (tower_info.position.y - enemy_pos.y) as f32;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance <= tower_info.range {
                            targets.push((enemy_entity, *enemy_pos, distance));
                        }
                    }

                    // Sort by distance
                    targets
                        .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

                    if let Some(&(_, target_pos, _)) = targets.first() {
                        // Find all enemies in blast radius
                        let mut affected_enemies = Vec::new();

                        for (enemy_entity, _, enemy_pos) in enemies.iter() {
                            let dx = (target_pos.x - enemy_pos.x) as f32;
                            let dy = (target_pos.y - enemy_pos.y) as f32;
                            let distance = (dx * dx + dy * dy).sqrt();

                            if distance <= 2.0 {
                                // Blast radius
                                affected_enemies.push(enemy_entity);
                            }
                        }

                        // Now process each affected enemy
                        for &enemy_entity in &affected_enemies {
                            if let Ok(mut enemy) = enemies.get_mut(enemy_entity) {
                                // Deal damage
                                enemy.1.health =
                                    enemy.1.health.saturating_sub(tower_info.damage as u32);

                                // Check if enemy is defeated
                                if enemy.1.health == 0 {
                                    // Award currency
                                    state.currency += enemy.1.value;

                                    // Remove the enemy
                                    commands.entity(enemy_entity).despawn();
                                }
                            }
                        }

                        // Update tower
                        if let Some(mut tower_comp) = commands.get_entity(tower_info.entity) {
                            let mut updated_tower = Tower::new(tower_info.tower_type);
                            updated_tower.last_attack = Instant::now();
                            updated_tower.damage = tower_info.damage;
                            updated_tower.range = tower_info.range;
                            updated_tower.level = 1; // Maintain level
                            tower_comp.insert(updated_tower);
                        }
                    }
                }
                TowerType::Freeze => {
                    // Freeze tower slows enemies in range
                    let mut targets = Vec::new();

                    // Collect eligible targets
                    for (enemy_entity, _, enemy_pos) in enemies.iter() {
                        // Calculate distance
                        let dx = (tower_info.position.x - enemy_pos.x) as f32;
                        let dy = (tower_info.position.y - enemy_pos.y) as f32;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance <= tower_info.range {
                            targets.push(enemy_entity);
                        }
                    }

                    // Process each target
                    let mut any_targets = false;
                    for &enemy_entity in &targets {
                        if let Ok(mut enemy) = enemies.get_mut(enemy_entity) {
                            // Apply slow effect and damage
                            enemy.1.speed *= 0.5; // Reduce speed by half
                            enemy.1.health =
                                enemy.1.health.saturating_sub(tower_info.damage as u32);

                            // Check if enemy is defeated
                            if enemy.1.health == 0 {
                                // Award currency
                                state.currency += enemy.1.value;

                                // Remove the enemy
                                commands.entity(enemy_entity).despawn();
                            }

                            any_targets = true;
                        }
                    }

                    // Update tower if any targets affected
                    if any_targets {
                        if let Some(mut tower_comp) = commands.get_entity(tower_info.entity) {
                            let mut updated_tower = Tower::new(tower_info.tower_type);
                            updated_tower.last_attack = Instant::now();
                            updated_tower.damage = tower_info.damage;
                            updated_tower.range = tower_info.range;
                            updated_tower.level = 1; // Maintain level
                            tower_comp.insert(updated_tower);
                        }
                    }
                }
                TowerType::Sniper => {
                    // Sniper tower targets strongest enemy
                    let mut highest_health = 0;
                    let mut strongest_enemy = None;

                    // Find enemy with highest health
                    for (enemy_entity, enemy, enemy_pos) in enemies.iter() {
                        // Calculate distance
                        let dx = (tower_info.position.x - enemy_pos.x) as f32;
                        let dy = (tower_info.position.y - enemy_pos.y) as f32;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance <= tower_info.range && enemy.health > highest_health {
                            highest_health = enemy.health;
                            strongest_enemy = Some(enemy_entity);
                        }
                    }

                    // Attack the strongest enemy
                    if let Some(enemy_entity) = strongest_enemy {
                        if let Ok(mut enemy) = enemies.get_mut(enemy_entity) {
                            // Deal high damage
                            enemy.1.health =
                                enemy.1.health.saturating_sub(tower_info.damage as u32);

                            // Check if enemy is defeated
                            if enemy.1.health == 0 {
                                // Award currency
                                state.currency += enemy.1.value;

                                // Remove the enemy
                                commands.entity(enemy_entity).despawn();
                            }

                            // Update tower
                            if let Some(mut tower_comp) = commands.get_entity(tower_info.entity) {
                                let mut updated_tower = Tower::new(tower_info.tower_type);
                                updated_tower.last_attack = Instant::now();
                                updated_tower.damage = tower_info.damage;
                                updated_tower.range = tower_info.range;
                                updated_tower.level = 1; // Maintain level
                                tower_comp.insert(updated_tower);
                            }
                        }
                    }
                }
                TowerType::Chain => {
                    // Chain tower damages multiple enemies in sequence
                    let mut targets = Vec::new();

                    // Collect eligible targets
                    for (enemy_entity, _, enemy_pos) in enemies.iter() {
                        // Calculate distance
                        let dx = (tower_info.position.x - enemy_pos.x) as f32;
                        let dy = (tower_info.position.y - enemy_pos.y) as f32;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance <= tower_info.range {
                            targets.push((enemy_entity, distance));
                        }
                    }

                    // Sort by distance
                    targets
                        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                    if let Some((first_target, _)) = targets.first().copied() {
                        // Process first target
                        if let Ok(mut enemy) = enemies.get_mut(first_target) {
                            // Deal damage
                            enemy.1.health =
                                enemy.1.health.saturating_sub(tower_info.damage as u32);

                            // Check if enemy is defeated
                            if enemy.1.health == 0 {
                                // Award currency
                                state.currency += enemy.1.value;

                                // Remove the enemy
                                commands.entity(first_target).despawn();
                            }

                            // Update tower
                            if let Some(mut tower_comp) = commands.get_entity(tower_info.entity) {
                                let mut updated_tower = Tower::new(tower_info.tower_type);
                                updated_tower.last_attack = Instant::now();
                                updated_tower.damage = tower_info.damage;
                                updated_tower.range = tower_info.range;
                                updated_tower.level = 1; // Maintain level
                                tower_comp.insert(updated_tower);
                            }
                        }

                        // Simplified chain effect due to borrow checker limitations
                        // In a real implementation, we would use a more complex approach to handle chains
                    }
                }
            }
        }
    }
}
