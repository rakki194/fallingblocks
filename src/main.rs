#![warn(clippy::all, clippy::pedantic)]

mod app;
mod components;
mod game;
mod menu;
mod menu_types;
mod particles;
mod screenshake;
mod sound;
mod systems;
mod tower_defense;
mod tower_defense_systems;
mod ui;
mod ui_tower_defense;

use std::io;
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use app::{App, AppResult};
use bevy_ecs::prelude::World;
use components::{GameState, Input};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fallingblocks::Time;
use log::{debug, error, info};
use menu_types::MenuState;
use particles::update_particles;
use ratatui::{Terminal, prelude::*};
use screenshake::update_screen_shake;
use sound::{AudioState, SoundEffect};
use systems::{game_tick_system, input_system};

fn update_audio_from_input(
    world: &mut World,
    toggle_music: bool,
    volume_up: bool,
    volume_down: bool,
) {
    if let Some(mut audio_state) = world.get_resource_mut::<AudioState>() {
        if toggle_music {
            audio_state.toggle_music();
        }
        if volume_up {
            audio_state.increase_volume(0.1);
        }
        if volume_down {
            audio_state.decrease_volume(0.1);
        }
    }
}

fn main() -> AppResult<()> {
    // Create log file and redirect stderr to it
    let log_path = "fallingblocks.log";
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .expect("Failed to create log file");

    // Redirect stderr to the log file
    let stderr_handle = std::io::stderr();
    let stderr_fd = stderr_handle.as_raw_fd();
    let log_file_fd = log_file.as_raw_fd();

    // Safety: We're redirecting stderr to our log file using standard POSIX operations
    unsafe {
        libc::dup2(log_file_fd, stderr_fd);
    }

    // Set RUST_BACKTRACE environment variable for detailed panic messages
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    // Configure the logger to use stderr (which is now redirected to our file)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    info!("Starting Tetris");

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let tick_rate = Duration::from_millis(33); // ~30 FPS
    let game_tick_rate = Duration::from_millis(50); // Game logic updates less often

    let mut app = App::new();

    // Initialize the world with required resources
    app.world.insert_resource(AudioState::new());

    // Check if audio device is available and inform the user
    if let Some(audio_state) = app.world.get_resource::<AudioState>() {
        if !audio_state.has_audio_device() {
            info!("No audio device available. Sound will be disabled.");
        }
    }

    let res = run_app(&mut terminal, app, tick_rate, game_tick_rate);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("Game error: {err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
    game_tick_rate: Duration,
) -> AppResult<()> {
    let mut last_render = Instant::now();
    let mut last_game_tick = Instant::now();

    // Initialize the Time resource
    app.world.insert_resource(Time::new());
    // Initialize Input resource with explicitly cleared state
    app.world.insert_resource(Input::default());
    // Initialize CoyoteTime resource
    app.world.insert_resource(components::CoyoteTime::default());

    // Check if we're using a mock device and inform the user
    if let Some(audio_state) = app.world.get_resource::<AudioState>() {
        if !audio_state.has_audio_device() {
            info!("Using audio fallback. Some sound features may be unavailable.");
        }
    }

    // Explicitly flush any pending input events that might be in the buffer
    while crossterm::event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    // Set the hard_drop_released flag to true initially
    {
        let mut input = app.world.resource_mut::<Input>();
        input.hard_drop_released = true;
    }

    debug!("Resources initialized");

    loop {
        // Draw the UI
        if last_render.elapsed() >= tick_rate {
            terminal.draw(|f| ui::render(f, &mut app))?;
            last_render = Instant::now();
        }

        // Process keyboard input
        if crossterm::event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                debug!("Key event: {key:?}");

                // Check for key release events
                if key.kind == event::KeyEventKind::Release {
                    // Track key releases for key-repeat prevention
                    let mut input = app.world.resource_mut::<Input>();
                    if key.code == KeyCode::Enter {
                        input.hard_drop_released = true;
                    }
                    continue; // Skip the rest of the input processing for release events
                }

                // First check if we need to handle game over state
                let is_game_over = {
                    let game_state = app.world.resource::<GameState>();
                    game_state.game_over
                };

                // Allow quitting with 'q' regardless of game state
                if key.code == KeyCode::Char('q') {
                    app.should_quit = true;
                    continue; // Skip the rest of the input processing
                }

                // Handle menu navigation when not in game
                if app.menu.state != menu_types::MenuState::Game
                    && app.menu.state != menu_types::MenuState::TowerDefense
                {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('w') => {
                            app.menu_renderer.prev_option(&mut app.menu);
                        }
                        KeyCode::Down | KeyCode::Char('s') => {
                            app.menu_renderer.next_option(&mut app.menu);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            // Check for quit option
                            if app.menu.state == menu_types::MenuState::MainMenu
                                && matches!(app.menu.selected_option, menu_types::MenuOption::Quit)
                            {
                                app.should_quit = true;
                            } else {
                                // Handle menu selection based on current state and option
                                let selected_option = app.menu.selected_option;

                                app.menu_renderer.handle_menu_selection(
                                    &mut app.menu,
                                    selected_option,
                                    &mut app.world,
                                );

                                // Reset game state if needed
                                if app.menu.state == menu_types::MenuState::Game
                                    || app.menu.state == menu_types::MenuState::TowerDefense
                                {
                                    app.reset();
                                }
                            }
                        }
                        _ => {}
                    }
                    continue; // Skip the rest of the input processing
                }

                // Handle game over state for both regular game and tower defense
                if is_game_over {
                    if key.code == KeyCode::Enter {
                        // Reset the game
                        app.reset();
                    }
                    continue; // Skip the rest of the input processing
                }

                // Handle different input based on game mode
                if app.menu.state == menu_types::MenuState::Game {
                    // Copy input flags before borrowing audio state
                    let toggle_music;
                    let volume_up;
                    let volume_down;
                    {
                        let input = app.world.resource::<Input>();
                        toggle_music = input.toggle_music;
                        volume_up = input.volume_up;
                        volume_down = input.volume_down;
                    }

                    // Update audio with copied flags
                    update_audio_from_input(&mut app.world, toggle_music, volume_up, volume_down);

                    // Check if the game is over
                    let is_game_over = {
                        let game_state = app.world.resource::<GameState>();
                        game_state.game_over
                    };

                    if !is_game_over {
                        // Process inputs directly
                        systems::input_system(&mut app.world);

                        // Handle coyote time
                        let mut coyote_time = app.world.resource_mut::<components::CoyoteTime>();
                        if coyote_time.active {
                            coyote_time.timer += 0.016; // ~60fps
                            if coyote_time.timer > 0.1 {
                                // 100ms coyote time window
                                coyote_time.active = false;
                                coyote_time.timer = 0.0;
                            }
                        }

                        // Run main game loop directly
                        systems::game_tick_system(&mut app.world, 0.016);

                        // Update particles and screen shake
                        update_particles(&mut app.world, 0.016);
                        update_screen_shake(&mut app.world, 0.016);

                        // Update app state from game state
                        app.sync_game_state();
                    }
                } else if app.menu.state == menu_types::MenuState::TowerDefense {
                    // Copy input flags before borrowing audio state
                    let toggle_music;
                    let volume_up;
                    let volume_down;
                    {
                        let input = app.world.resource::<Input>();
                        toggle_music = input.toggle_music;
                        volume_up = input.volume_up;
                        volume_down = input.volume_down;
                    }

                    // Update audio with copied flags
                    update_audio_from_input(&mut app.world, toggle_music, volume_up, volume_down);

                    // Check if the tower defense game is over
                    let is_game_over = {
                        if let Some(state) = app
                            .world
                            .get_resource::<crate::tower_defense::TowerDefenseState>()
                        {
                            state.game_over
                        } else {
                            false
                        }
                    };

                    // Handle tower defense specific keyboard inputs
                    match key.code {
                        // Tower selection with number keys
                        KeyCode::Char('1') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                state.selected_tower_type =
                                    Some(crate::tower_defense::TowerType::Basic);
                            }
                        }
                        KeyCode::Char('2') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                state.selected_tower_type =
                                    Some(crate::tower_defense::TowerType::Cannon);
                            }
                        }
                        KeyCode::Char('3') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                state.selected_tower_type =
                                    Some(crate::tower_defense::TowerType::Freeze);
                            }
                        }
                        KeyCode::Char('4') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                state.selected_tower_type =
                                    Some(crate::tower_defense::TowerType::Sniper);
                            }
                        }
                        KeyCode::Char('5') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                state.selected_tower_type =
                                    Some(crate::tower_defense::TowerType::Chain);
                            }
                        }
                        // Start wave
                        KeyCode::Char('w') => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                if !state.wave_in_progress && !state.game_over {
                                    state.wave_in_progress = true;
                                    state.enemies_spawned = 0;
                                    state.enemies_to_spawn = 10 + (state.wave * 2); // Increase enemies per wave
                                    state.wave_completed = false;
                                    state.next_enemy_spawn = std::time::Instant::now();
                                }
                            }
                        }
                        // Tower placement
                        KeyCode::Char(' ') => {
                            // Get cursor position and tower selection info
                            let mut cursor_pos = crate::components::Position { x: 5, y: 5 }; // Default fallback
                            let mut can_place = false;
                            let tower_type;
                            let tower_cost;

                            {
                                if let Some(state) =
                                    app.world
                                        .get_resource::<crate::tower_defense::TowerDefenseState>()
                                {
                                    // Use the cursor position from state - Position uses i32 so no casting needed
                                    cursor_pos = crate::components::Position {
                                        x: state.cursor_x,
                                        y: state.cursor_y,
                                    };

                                    if let Some(selected_type) = state.selected_tower_type {
                                        tower_type = Some(selected_type);
                                        tower_cost = selected_type.get_cost();
                                        can_place = state.currency >= tower_cost;
                                    } else {
                                        can_place = false;
                                        tower_type = None;
                                        tower_cost = 0;
                                    }
                                } else {
                                    can_place = false;
                                    tower_type = None;
                                    tower_cost = 0;
                                }
                            }

                            // Check if we can place tower
                            if can_place && tower_type.is_some() {
                                if let Ok(true) = can_place_tower(&mut app.world, &cursor_pos) {
                                    // Deduct cost
                                    if let Some(mut state) = app.world.get_resource_mut::<crate::tower_defense::TowerDefenseState>() {
                                        state.currency -= tower_cost;
                                    }

                                    // Create tower entity
                                    let tower =
                                        crate::tower_defense::Tower::new(tower_type.unwrap());
                                    app.world.spawn((tower, cursor_pos));

                                    // Play sound effect
                                    if let Some(audio_state) =
                                        app.world.get_resource_mut::<AudioState>()
                                    {
                                        audio_state.play_sound(SoundEffect::Place);
                                    }
                                }
                            }
                        }
                        // Navigation
                        KeyCode::Left => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                if state.cursor_x > 1 {
                                    state.cursor_x -= 1;
                                }
                                // Handle map scrolling with Shift+arrow keys
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    if state.scroll_x > 0 {
                                        state.scroll_x -= 1;
                                    }
                                }
                            }
                        }
                        KeyCode::Right => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                // Check board bounds before moving
                                if state.cursor_x < (state.map_width - 2) {
                                    state.cursor_x += 1;
                                }
                                // Handle map scrolling with Shift+arrow keys
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    let max_scroll = state.map_width - 20; // Assuming viewport width of 20
                                    if state.scroll_x < max_scroll {
                                        state.scroll_x += 1;
                                    }
                                }
                            }
                        }
                        KeyCode::Up => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                if state.cursor_y > 1 {
                                    state.cursor_y -= 1;
                                }
                                // Handle map scrolling with Shift+arrow keys
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    if state.scroll_y > 0 {
                                        state.scroll_y -= 1;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let Some(mut state) =
                                app.world
                                    .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                            {
                                if state.cursor_y < (state.map_height - 2) {
                                    state.cursor_y += 1;
                                }
                                // Handle map scrolling with Shift+arrow keys
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    let max_scroll = state.map_height - 20; // Assuming viewport height of 20
                                    if state.scroll_y < max_scroll {
                                        state.scroll_y += 1;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    if !is_game_over {
                        // Run tower defense specific systems
                        if let Some(mut state) = app
                            .world
                            .get_resource_mut::<crate::tower_defense::TowerDefenseState>()
                        {
                            if state.wave_in_progress {
                                // Release state to avoid borrow conflicts
                                drop(state);

                                // Spawn enemies using our new system
                                tower_defense_systems::spawn_enemies(&mut app.world);
                            }
                        }

                        // Check wave completion
                        tower_defense_systems::check_wave_completion(&mut app.world);

                        // Process enemies and towers
                        tower_defense_systems::process_enemy_movement(&mut app.world);
                        tower_defense_systems::process_tower_attacks(&mut app.world);

                        // Screen shake and particles
                        update_particles(&mut app.world, 0.016); // ~60 FPS
                        update_screen_shake(&mut app.world, 0.016);
                    }
                }

                // Update last key in game state
                let mut game_state = app.world.resource_mut::<GameState>();
                game_state.last_key = Some(key);
            }
        }

        // Do game updates at a fixed rate
        if last_game_tick.elapsed() >= game_tick_rate {
            let delta = last_game_tick.elapsed().as_secs_f32();
            last_game_tick = Instant::now();

            // Update the time resource with the new delta
            {
                let mut time = app.world.resource_mut::<Time>();
                time.update();
            }

            match app.menu.state {
                MenuState::Game => {
                    // Update game state
                    app.on_tick();
                    input_system(&mut app.world);
                    game_tick_system(&mut app.world, delta);
                }
                MenuState::TowerDefense => {
                    // Update tower defense state
                    app.on_tick();

                    // Process enemy movement
                    crate::tower_defense_systems::process_enemy_movement(&mut app.world);

                    // Process tower attacks
                    crate::tower_defense_systems::process_tower_attacks(&mut app.world);

                    // Spawn new enemies if wave is in progress
                    crate::tower_defense_systems::spawn_enemies(&mut app.world);

                    // Check for wave completion
                    crate::tower_defense_systems::check_wave_completion(&mut app.world);

                    // Process particles and screen shake
                    particles::update_particles(&mut app.world, delta);
                    screenshake::update_screen_shake(&mut app.world, delta);
                }
                _ => {
                    // No updates needed for menu screens
                    // Animations or transitions would go here if needed
                }
            }
        }

        if app.should_quit {
            info!("Game quit by user");
            return Ok(());
        }
    }
}

// Placeholder function to check if a tower can be placed at a given position
fn can_place_tower(
    world: &mut World,
    position: &crate::components::Position,
) -> Result<bool, String> {
    // Get a copy of the segments we need to check
    let segments_to_check = if let Some(path) =
        world.get_resource::<fallingblocks::tower_defense::TowerDefensePath>()
    {
        path.segments.clone()
    } else {
        Vec::new()
    };

    // Check if position is on a path segment
    if !segments_to_check.is_empty() {
        let mut segments_query = world.query::<&fallingblocks::tower_defense::PathSegment>();

        for segment_entity in segments_to_check {
            if let Ok(segment) = segments_query.get(world, segment_entity) {
                if segment.position.x == position.x && segment.position.y == position.y {
                    return Ok(false); // Can't place on path
                }
            }
        }
    }

    // Check if there's already a tower at this position
    let mut towers_query = world.query::<(
        &fallingblocks::tower_defense::Tower,
        &crate::components::Position,
    )>();
    for (_, tower_pos) in towers_query.iter(world) {
        if tower_pos.x == position.x && tower_pos.y == position.y {
            return Ok(false); // Can't place on another tower
        }
    }

    // Position is valid for tower placement
    Ok(true)
}
