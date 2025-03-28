#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use crate::app::App;
use crate::components::Position;
use crate::tower_defense::{
    Enemy, PathSegment, Tower, TowerDefensePath, TowerDefenseState, TowerType,
};
use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

/// Main rendering function for tower defense mode
pub fn render_tower_defense(f: &mut Frame, app: &mut App) {
    // Get available area
    let area = f.area();

    // Ensure we have enough space for the game
    if area.width < 60 || area.height < 30 {
        render_too_small_warning(f, area);
        return;
    }

    // Create the layout
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Game board area (left panel)
    let game_area = main_layout[0];

    // Info panel area (right panel)
    let info_area = main_layout[1];

    // Create info panel layout
    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(8),  // Stats
            Constraint::Length(10), // Tower selection
            Constraint::Min(5),     // Tower info
        ])
        .split(info_area);

    // Render title
    let title = Paragraph::new("TETRIS TOWER DEFENSE")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, info_layout[0]);

    // Render game stats
    render_game_stats(f, app, info_layout[1]);

    // Render tower selection
    render_tower_selection(f, app, info_layout[2]);

    // Render selected tower info
    render_tower_info(f, app, info_layout[3]);

    // Render the game board
    render_tower_defense_game(f, app, game_area);
}

/// Render a warning when the terminal is too small
fn render_too_small_warning(f: &mut Frame, area: Rect) {
    let warning = Paragraph::new("Terminal too small!\nPlease resize to at least 60x30.")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Warning"));

    let warning_area = centered_rect(50, 30, area);
    f.render_widget(warning, warning_area);
}

/// Render the tower defense game stats
fn render_game_stats(f: &mut Frame, app: &App, area: Rect) {
    if let Some(state) = app.world.get_resource::<TowerDefenseState>() {
        // Calculate enemies destroyed without risking overflow
        let enemies_destroyed = if state.enemies_spawned <= state.enemies_to_spawn {
            state.enemies_spawned
        } else {
            state.enemies_to_spawn
        };

        let stats_text = format!(
            "Wave: {}\nLives: {}\nCurrency: {} coins\nEnemies: {}/{}",
            state.wave, state.lives, state.currency, enemies_destroyed, state.enemies_to_spawn
        );

        let stats =
            Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title("Stats"));

        f.render_widget(stats, area);
    }
}

/// Render the tower selection panel
fn render_tower_selection(f: &mut Frame, app: &App, area: Rect) {
    let tower_types = [
        TowerType::Basic,
        TowerType::Cannon,
        TowerType::Freeze,
        TowerType::Sniper,
        TowerType::Chain,
    ];

    let selected_type = app
        .world
        .get_resource::<TowerDefenseState>()
        .and_then(|state| state.selected_tower_type);

    let title = "Towers";
    let block = Block::default().borders(Borders::ALL).title(title);

    f.render_widget(block, area);

    let inner_area = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    let tower_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            tower_types
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(inner_area);

    for (i, tower_type) in tower_types.iter().enumerate() {
        let cost = tower_type.get_cost();
        let name = tower_type.get_name();
        let color = tower_type.get_color();

        let is_selected = selected_type == Some(*tower_type);
        let has_enough_money = app
            .world
            .get_resource::<TowerDefenseState>()
            .map_or(false, |state| state.currency >= cost);

        let style = if is_selected {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else if !has_enough_money {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(color)
        };

        let tower_text = format!("{}: {} coins", name, cost);
        let tower_item = Paragraph::new(tower_text).style(style);

        f.render_widget(tower_item, tower_layout[i]);
    }
}

/// Render information about the selected tower
fn render_tower_info(f: &mut Frame, app: &App, area: Rect) {
    if let Some(state) = app.world.get_resource::<TowerDefenseState>() {
        if let Some(tower_type) = state.selected_tower_type {
            let name = tower_type.get_name();
            let description = tower_type.get_description();
            let cost = tower_type.get_cost();
            let color = tower_type.get_color();

            let tower_text = format!(
                "{}\n\n{}\n\nCost: {} coins\n\nPress SPACE to place",
                name, description, cost
            );

            let info = Paragraph::new(tower_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tower Info")
                        .style(Style::default().fg(color)),
                )
                .wrap(Wrap { trim: true });

            f.render_widget(info, area);
        } else {
            let info = Paragraph::new("Select a tower with 1-5")
                .block(Block::default().borders(Borders::ALL).title("Tower Info"));

            f.render_widget(info, area);
        }
    }
}

/// Render the tower defense game board
pub fn render_tower_defense_game(f: &mut Frame, app: &mut App, area: Rect) {
    // Adjust the board size to have larger cells
    // Each cell should be at least 3x2 characters to better render tetrominos
    let cell_width = 3;
    let cell_height = 2;

    // Get scroll offset from game state
    let (scroll_x, scroll_y) = if let Some(state) = app.world.get_resource::<TowerDefenseState>() {
        (state.scroll_x, state.scroll_y)
    } else {
        (0, 0) // Default if no state
    };

    // Visible grid dimensions based on available area
    let visible_width = (area.width / cell_width).min(20);
    let visible_height = (area.height / cell_height).min(20);

    // Create a block for the game area
    let game_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Tower Defense")
        .title_alignment(Alignment::Center);

    // Borrow game_block instead of moving it
    f.render_widget(&game_block, area);

    // Adjust the inner area to account for the border
    let inner_area = game_block.inner(area);

    // Render the path
    render_path(
        f,
        app,
        inner_area,
        cell_width,
        cell_height,
        scroll_x,
        scroll_y,
    );

    // Render towers
    render_towers(
        f,
        app,
        inner_area,
        cell_width,
        cell_height,
        scroll_x,
        scroll_y,
    );

    // Render enemies
    render_enemies(
        f,
        app,
        inner_area,
        cell_width,
        cell_height,
        scroll_x,
        scroll_y,
    );

    // Render cursor
    render_placement_cursor(
        f,
        app,
        inner_area,
        cell_width,
        cell_height,
        scroll_x,
        scroll_y,
    );

    // Render scroll indicators if needed
    if let Some(state) = app.world.get_resource::<TowerDefenseState>() {
        let can_scroll_left = scroll_x > 0;
        let can_scroll_right = scroll_x < state.map_width - visible_width as i32;
        let can_scroll_up = scroll_y > 0;
        let can_scroll_down = scroll_y < state.map_height - visible_height as i32;

        if can_scroll_left {
            let left_indicator = Paragraph::new("◄")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);
            f.render_widget(
                left_indicator,
                Rect::new(area.x, area.y + area.height / 2, 1, 1),
            );
        }

        if can_scroll_right {
            let right_indicator = Paragraph::new("►")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Right);
            f.render_widget(
                right_indicator,
                Rect::new(area.x + area.width - 1, area.y + area.height / 2, 1, 1),
            );
        }

        if can_scroll_up {
            let up_indicator = Paragraph::new("▲")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);
            f.render_widget(
                up_indicator,
                Rect::new(area.x + area.width / 2, area.y, 1, 1),
            );
        }

        if can_scroll_down {
            let down_indicator = Paragraph::new("▼")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);
            f.render_widget(
                down_indicator,
                Rect::new(area.x + area.width / 2, area.y + area.height - 1, 1, 1),
            );
        }
    }
}

/// Render the path for tower defense
fn render_path(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
    scroll_x: i32,
    scroll_y: i32,
) {
    // First get the path resource
    let path_opt = app.world.get_resource::<TowerDefensePath>().cloned();

    if let Some(path) = path_opt {
        // Create a collection of all path positions for easier lookup
        let mut path_positions = std::collections::HashMap::new();
        let mut is_main_path = std::collections::HashSet::new();

        // We need to identify which segments are on the main path (enemies follow) vs decorative
        let mut current_entity = path.start;

        // Store the start and end positions to render them differently
        let mut start_pos = None;
        let mut end_pos = None;

        // Collect the main path segments
        while let Some(segment) = app.world.get::<PathSegment>(current_entity) {
            let pos = (segment.position.x, segment.position.y);
            is_main_path.insert(pos);

            // Track start and end positions
            if current_entity == path.start {
                start_pos = Some(pos);
            } else if current_entity == path.end {
                end_pos = Some(pos);
            }

            if let Some(next) = segment.next_segment {
                current_entity = next;
            } else {
                // If no next segment, this is the end
                end_pos = Some(pos);
                break;
            }
        }

        // Collect all path segments
        for segment_entity in &path.segments {
            if let Some(segment) = app.world.get::<PathSegment>(*segment_entity) {
                let x = segment.position.x;
                let y = segment.position.y;
                path_positions.insert((x, y), segment.next_segment);
            }
        }

        // For each path position, determine what type of path segment it is
        // for better visual representation
        for ((x, y), next_segment) in path_positions {
            let visible_x = x - scroll_x;
            let visible_y = y - scroll_y;

            // Skip if outside visible area
            if visible_x < 0
                || visible_y < 0
                || visible_x >= area.width as i32 / cell_width as i32
                || visible_y >= area.height as i32 / cell_height as i32
            {
                continue;
            }

            // Render the path segment with adjusted coordinates
            let screen_x = visible_x as u16;
            let screen_y = visible_y as u16;

            // Start position gets a special appearance
            if start_pos == Some((x as i32, y as i32)) {
                let start_char = "S";
                let start_segment = Paragraph::new(start_char).style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                );

                let pos = Rect {
                    x: area.x + screen_x * cell_width,
                    y: area.y + screen_y * cell_height,
                    width: cell_width,
                    height: cell_height,
                };

                f.render_widget(start_segment, pos);
                continue;
            }

            // End position gets a special appearance
            if end_pos == Some((x as i32, y as i32)) {
                let end_char = "E";
                let end_segment = Paragraph::new(end_char)
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

                let pos = Rect {
                    x: area.x + screen_x * cell_width,
                    y: area.y + screen_y * cell_height,
                    width: cell_width,
                    height: cell_height,
                };

                f.render_widget(end_segment, pos);
                continue;
            }

            // Determine if this is a main path or decorative path segment
            let is_on_main_path = is_main_path.contains(&(x as i32, y as i32));

            // Choose path appearance based on whether it's main path or decorative
            let (path_char, style) = if is_on_main_path {
                // Main path segments - enemies follow these
                // Use directional characters if we can determine direction
                if let Some(next) = next_segment {
                    if let Some(next_segment) = app.world.get::<PathSegment>(next) {
                        let next_x = next_segment.position.x as i32;
                        let next_y = next_segment.position.y as i32;
                        let curr_x = x as i32;
                        let curr_y = y as i32;

                        // Determine direction based on next segment
                        let dir_char = if next_x > curr_x {
                            "→" // Right
                        } else if next_x < curr_x {
                            "←" // Left
                        } else if next_y > curr_y {
                            "↓" // Down
                        } else {
                            "↑" // Up
                        };

                        (
                            dir_char,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ("●", Style::default().fg(Color::Yellow))
                    }
                } else {
                    ("●", Style::default().fg(Color::Yellow))
                }
            } else {
                // Decorative path segments - just for width
                // Use different style based on position for visual interest
                let path_variants = ["·", "░", "▒"];
                let variant_idx = (x as usize + y as usize) % path_variants.len();
                let path_variant = path_variants[variant_idx];

                (path_variant, Style::default().fg(Color::DarkGray))
            };

            let path_segment = Paragraph::new(path_char).style(style);

            let pos = Rect {
                x: area.x + screen_x * cell_width,
                y: area.y + screen_y * cell_height,
                width: cell_width,
                height: cell_height,
            };

            f.render_widget(path_segment, pos);
        }
    }
}

/// Tower base stats for UI rendering (duplicates Tower::new stats for UI access)
struct TowerUIStats {
    range: f32,
}

/// Get stats for a tower type - refactored to avoid duplication
fn get_tower_stats(tower_type: TowerType) -> TowerUIStats {
    match tower_type {
        TowerType::Basic => TowerUIStats { range: 3.0 },
        TowerType::Cannon => TowerUIStats { range: 2.0 },
        TowerType::Freeze => TowerUIStats { range: 2.5 },
        TowerType::Sniper => TowerUIStats { range: 5.0 },
        TowerType::Chain => TowerUIStats { range: 3.0 },
    }
}

/// Render towers on the game board
fn render_towers(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
    scroll_x: i32,
    scroll_y: i32,
) {
    // Collect tower data once
    struct TowerData {
        tower_type: TowerType,
        x: i32,
        y: i32,
    }

    let towers: Vec<TowerData> = app
        .world
        .query::<(&Tower, &Position)>()
        .iter(&app.world)
        .map(|(tower, pos)| TowerData {
            tower_type: tower.tower_type,
            x: pos.x,
            y: pos.y,
        })
        .collect();

    // Render each tower with a distinctive symbol based on type
    for tower in towers {
        // Apply scroll offset
        let visible_x = tower.x - scroll_x;
        let visible_y = tower.y - scroll_y;

        // Skip if outside visible area
        if visible_x < 0
            || visible_y < 0
            || visible_x >= area.width as i32 / cell_width as i32
            || visible_y >= area.height as i32 / cell_height as i32
        {
            continue;
        }

        let screen_x = visible_x as u16;
        let screen_y = visible_y as u16;

        let (tower_char, style) = get_tower_visual(tower.tower_type);

        let tower_widget = Paragraph::new(tower_char).style(style);

        let pos = Rect {
            x: area.x + screen_x * cell_width,
            y: area.y + screen_y * cell_height,
            width: cell_width,
            height: cell_height,
        };

        f.render_widget(tower_widget, pos);
    }
}

/// Get the visual representation for a tower type
fn get_tower_visual(tower_type: TowerType) -> (&'static str, Style) {
    match tower_type {
        TowerType::Basic => (
            "↟",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        TowerType::Cannon => (
            "⊛",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        TowerType::Freeze => (
            "❄",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        TowerType::Sniper => (
            "✜",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        TowerType::Chain => (
            "⚡",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    }
}

/// Get visual representation for enemy
fn get_enemy_visual(
    tetromino_type: crate::components::TetrominoType,
    is_boss: bool,
    is_armored: bool,
) -> (String, Style) {
    // Choose appropriate style based on tetromino type with armor/boss modifications
    let mut style = Style::default();

    // Base color by tetromino type
    let color = match tetromino_type {
        crate::components::TetrominoType::I => Color::Cyan,
        crate::components::TetrominoType::J => Color::Blue,
        crate::components::TetrominoType::L => Color::LightYellow,
        crate::components::TetrominoType::O => Color::Yellow,
        crate::components::TetrominoType::S => Color::Green,
        crate::components::TetrominoType::T => Color::Magenta,
        crate::components::TetrominoType::Z => Color::Red,
    };

    style = style.fg(color);

    // Add modifiers for special enemy types
    if is_boss {
        style = style.add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK);
    }

    if is_armored {
        style = style.add_modifier(Modifier::REVERSED);
    }

    // Choose appropriate character representation for each tetromino type
    // Use a more distinct visual for each type to make them recognizable
    let char_rep = match tetromino_type {
        crate::components::TetrominoType::I => "I█I", // Horizontal line
        crate::components::TetrominoType::J => "J┘ ", // J shape
        crate::components::TetrominoType::L => "L└ ", // L shape
        crate::components::TetrominoType::O => "■■ ", // Square
        crate::components::TetrominoType::S => "S≈ ", // S shape
        crate::components::TetrominoType::T => "T┴ ", // T shape
        crate::components::TetrominoType::Z => "Z≋ ", // Z shape
    };

    (char_rep.to_string(), style)
}

/// Get visual representation for health bar
fn get_health_visual(health_percent: f32) -> (String, Color) {
    let color = if health_percent > 0.7 {
        Color::Green
    } else if health_percent > 0.3 {
        Color::Yellow
    } else {
        Color::Red
    };

    let bars = (health_percent * 3.0).ceil() as usize;
    let mut health_str = "█".repeat(bars);
    // Pad with spaces to maintain a consistent width
    while health_str.len() < 3 {
        health_str.push(' ');
    }

    (health_str, color)
}

/// Render enemies on the game board
fn render_enemies(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
    scroll_x: i32,
    scroll_y: i32,
) {
    // Collect enemy data once
    struct EnemyData {
        tetromino_type: crate::components::TetrominoType,
        is_boss: bool,
        is_armored: bool,
        x: i32,
        y: i32,
        health_percent: f32,
    }

    let enemies: Vec<EnemyData> = app
        .world
        .query::<(&Enemy, &Position)>()
        .iter(&app.world)
        .map(|(enemy, pos)| {
            let health_percent = if enemy.max_health > 0 {
                enemy.health as f32 / enemy.max_health as f32
            } else {
                1.0
            };

            EnemyData {
                tetromino_type: enemy.tetromino_type,
                is_boss: enemy.is_boss,
                is_armored: enemy.is_armored,
                x: pos.x,
                y: pos.y,
                health_percent,
            }
        })
        .collect();

    // Render each enemy with a distinctive appearance
    for enemy in enemies {
        // Apply scroll offset
        let visible_x = enemy.x - scroll_x;
        let visible_y = enemy.y - scroll_y;

        // Skip if outside visible area
        if visible_x < 0
            || visible_y < 0
            || visible_x >= area.width as i32 / cell_width as i32
            || visible_y >= area.height as i32 / cell_height as i32
        {
            continue;
        }

        let screen_x = visible_x as u16;
        let screen_y = visible_y as u16;

        // Get enemy visual representation
        let (enemy_char, style) =
            get_enemy_visual(enemy.tetromino_type, enemy.is_boss, enemy.is_armored);

        // Render the enemy
        let enemy_widget = Paragraph::new(enemy_char).style(style);

        // Position for the enemy
        let pos = Rect {
            x: area.x + screen_x * cell_width,
            y: area.y + screen_y * cell_height,
            width: cell_width,
            height: cell_height,
        };

        f.render_widget(enemy_widget, pos);

        // Draw a health bar below the enemy if it's not at full health
        if enemy.health_percent < 1.0 {
            // Position for health bar
            let health_bar_pos = Rect {
                x: area.x + screen_x * cell_width,
                y: area.y + screen_y * cell_height + 1,
                width: cell_width,
                height: 1,
            };

            // Get health bar visual representation
            let (health_char, health_color) = get_health_visual(enemy.health_percent);
            let health_widget =
                Paragraph::new(health_char).style(Style::default().fg(health_color));

            // Only render if within bounds
            if health_bar_pos.y < area.height {
                f.render_widget(health_widget, health_bar_pos);
            }
        }
    }
}

/// Render the cursor for tower placement
fn render_placement_cursor(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
    scroll_x: i32,
    scroll_y: i32,
) {
    if let Some(state) = app.world.get_resource::<TowerDefenseState>() {
        // Only show cursor if a tower is selected
        if let Some(tower_type) = state.selected_tower_type {
            let cursor_x = state.cursor_x;
            let cursor_y = state.cursor_y;

            // Apply scroll offset
            let visible_x = cursor_x - scroll_x;
            let visible_y = cursor_y - scroll_y;

            // Skip if outside visible area
            if visible_x < 0
                || visible_y < 0
                || visible_x >= area.width as i32 / cell_width as i32
                || visible_y >= area.height as i32 / cell_height as i32
            {
                return;
            }

            let screen_x = visible_x as u16;
            let screen_y = visible_y as u16;

            // Check if we can place a tower at the cursor position
            let can_place = is_valid_tower_placement(app, cursor_x as i32, cursor_y as i32);

            // Get tower color based on tower type
            let color = tower_type.get_color();

            // Set cursor style based on placement validity
            let cursor_style = if can_place {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            };

            // Draw cursor
            let cursor_char = if can_place { "◎" } else { "⊗" };
            let cursor_widget = Paragraph::new(cursor_char).style(cursor_style);

            let cursor_pos = Rect {
                x: area.x + screen_x * cell_width,
                y: area.y + screen_y * cell_height,
                width: cell_width,
                height: cell_height,
            };

            f.render_widget(cursor_widget, cursor_pos);

            // Show tower range as a circle around the cursor only if placement is valid
            if can_place {
                // Get tower stats
                let tower_stats = get_tower_stats(tower_type);
                let tower_range = tower_stats.range;

                // Convert range to integer radius for rendering
                let range_radius = tower_range.ceil() as i32;

                // Draw range indicator circle
                for dy in -range_radius..=range_radius {
                    for dx in -range_radius..=range_radius {
                        // Skip if outside range circle
                        let distance = ((dx * dx + dy * dy) as f32).sqrt();
                        if distance > tower_range {
                            continue;
                        }

                        // Calculate position
                        let range_x = cursor_x as i32 + dx;
                        let range_y = cursor_y as i32 + dy;

                        // Apply scroll offset
                        let visible_range_x = range_x - scroll_x;
                        let visible_range_y = range_y - scroll_y;

                        // Skip if outside visible area
                        if visible_range_x < 0
                            || visible_range_y < 0
                            || visible_range_x >= area.width as i32 / cell_width as i32
                            || visible_range_y >= area.height as i32 / cell_height as i32
                        {
                            continue;
                        }

                        let screen_range_x = visible_range_x as u16;
                        let screen_range_y = visible_range_y as u16;

                        // Skip the center point (where the tower will be)
                        if dx == 0 && dy == 0 {
                            continue;
                        }

                        // Calculate screen coordinates
                        let screen_x = area.x + screen_range_x * cell_width;
                        let screen_y = area.y + screen_range_y * cell_height;

                        // Final safety check - ensure coordinates are within buffer bounds
                        if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                            continue;
                        }

                        // Draw range indicator with a different character based on distance
                        let range_char = if (distance - tower_range).abs() < 0.5 {
                            "·" // Edge of range
                        } else {
                            "∙" // Interior of range
                        };

                        // Intensity based on distance (fade toward edges)
                        let intensity = 1.0 - (distance / tower_range).min(1.0);
                        // Since Color doesn't have with_alpha, use a dimmed version of the color instead
                        let range_style = if intensity > 0.7 {
                            Style::default().fg(color)
                        } else if intensity > 0.4 {
                            Style::default().fg(Color::DarkGray)
                        } else {
                            Style::default().fg(Color::Black)
                        };

                        let range_pos = Rect {
                            x: screen_x,
                            y: screen_y,
                            width: cell_width,
                            height: cell_height,
                        };

                        let range_widget = Paragraph::new(range_char).style(range_style);
                        f.render_widget(range_widget, range_pos);
                    }
                }
            }
        }
    }
}

// Helper function to check if a tower can be placed at a given position
fn is_valid_tower_placement(app: &mut App, x: i32, y: i32) -> bool {
    // Can't place outside bounds
    if x < 0 || y < 0 {
        return false;
    }

    // Check if there's already a tower at this position
    let tower_positions: Vec<(i32, i32)> = app
        .world
        .query::<(&Tower, &Position)>()
        .iter(&app.world)
        .map(|(_, pos)| (pos.x, pos.y))
        .collect();

    // Check if position is already occupied by a tower
    if tower_positions.iter().any(|&(tx, ty)| tx == x && ty == y) {
        return false;
    }

    // Check if this position is on the path
    let path_positions: Vec<(i32, i32)> = app
        .world
        .query::<&PathSegment>()
        .iter(&app.world)
        .map(|segment| (segment.position.x, segment.position.y))
        .collect();

    // Can't place towers on the path
    if path_positions.iter().any(|&(px, py)| px == x && py == y) {
        return false;
    }

    // If we got here, the position is valid
    true
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
