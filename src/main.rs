#![warn(clippy::all, clippy::pedantic)]

mod app;
mod components;
mod config;
mod game;
mod menu;
mod menu_types;
mod particles;
mod screenshake;
mod sound;
mod systems;
mod ui;

use std::io;
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use app::{App, AppResult};
use components::{Board, GameState, Input};
use crossterm::event::KeyCode;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fallingblocks::Time;
use log::{debug, error, info};
use ratatui::{Terminal, prelude::*};
use sound::AudioState;

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

    // Initialize configuration system
    if let Err(e) = config::loader::load_config_from_file() {
        error!("Failed to load configuration: {:?}", e);
        // Continue with default configuration
    } else {
        info!("Configuration loaded successfully");
    }

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let tick_rate = Duration::from_millis(33); // ~30 FPS
    let game_tick_rate = Duration::from_millis(50); // Game logic updates less often

    let app = App::new();
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
    // Initialize the AudioState resource
    app.world.insert_resource(AudioState::new());
    // Initialize CoyoteTime resource
    app.world.insert_resource(components::CoyoteTime::default());

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

    // Flag to track if we've already warned about audio device issues
    let mut audio_error_logged = false;

    loop {
        // Draw the UI
        if last_render.elapsed() >= tick_rate {
            terminal.draw(|f| ui::render(f, &mut app))?;
            last_render = Instant::now();
        }

        // Check if any systems need updating
        if last_game_tick.elapsed() >= game_tick_rate {
            let delta_seconds = last_game_tick.elapsed().as_secs_f32();
            last_game_tick = Instant::now();

            // Update time resource
            {
                let mut time = app.world.resource_mut::<Time>();
                time.update();
            }

            // Check audio system status
            let audio_unavailable = {
                let audio_state = app.world.resource::<AudioState>();
                !audio_state.is_audio_available()
            };

            // If audio device is unavailable and we haven't logged it yet, log it once
            if audio_unavailable && !audio_error_logged {
                error!("Audio device is unavailable. Continuing without sound.");
                audio_error_logged = true;
            }

            // Exit if needed
            if app.should_quit {
                return Ok(());
            }

            // Only run game systems if in game state
            if app.menu.state == menu_types::MenuState::Game {
                // Run game systems
                systems::input_system(&mut app.world);
                systems::game_tick_system(&mut app.world, delta_seconds);
                app.sync_game_state();
            }

            // Run general app update
            app.on_tick();
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
                if app.menu.state != menu_types::MenuState::Game {
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
                                // Store current state before borrowing
                                let menu_state = app.menu.state.clone();

                                // Handle main menu selection
                                if menu_state == menu_types::MenuState::MainMenu {
                                    if matches!(
                                        app.menu.selected_option,
                                        menu_types::MenuOption::NewGame
                                    ) {
                                        app.menu.state = menu_types::MenuState::Game;
                                        app.reset();
                                    } else if matches!(
                                        app.menu.selected_option,
                                        menu_types::MenuOption::Options
                                    ) {
                                        app.menu.state = menu_types::MenuState::Options;
                                    }
                                }
                                // Handle options menu selection
                                else if menu_state == menu_types::MenuState::Options {
                                    match app.menu.options_selected {
                                        menu_types::OptionsOption::MusicToggle => {
                                            if let Some(mut audio_state) =
                                                app.world.get_resource_mut::<AudioState>()
                                            {
                                                audio_state.toggle_music();
                                            }
                                        }
                                        menu_types::OptionsOption::SoundToggle => {
                                            if let Some(mut audio_state) =
                                                app.world.get_resource_mut::<AudioState>()
                                            {
                                                audio_state.toggle_sound();
                                            }
                                        }
                                        menu_types::OptionsOption::VolumeUp => {
                                            if let Some(mut audio_state) =
                                                app.world.get_resource_mut::<AudioState>()
                                            {
                                                let volume = audio_state.get_volume();
                                                audio_state.set_volume((volume + 0.1).min(1.0));
                                            }
                                        }
                                        menu_types::OptionsOption::VolumeDown => {
                                            if let Some(mut audio_state) =
                                                app.world.get_resource_mut::<AudioState>()
                                            {
                                                let volume = audio_state.get_volume();
                                                audio_state.set_volume((volume - 0.1).max(0.0));
                                            }
                                        }
                                        menu_types::OptionsOption::Back => {
                                            app.menu.state = menu_types::MenuState::MainMenu;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Esc => {
                            // Return to main menu if in options
                            if app.menu.state == menu_types::MenuState::Options {
                                app.menu.state = menu_types::MenuState::MainMenu;
                            }
                        }
                        // Process arrow key left/right for volume control
                        KeyCode::Left => {
                            if app.menu.state == menu_types::MenuState::Options
                                && matches!(
                                    app.menu.options_selected,
                                    menu_types::OptionsOption::VolumeDown
                                )
                            {
                                // Volume down
                                if let Some(mut audio_state) =
                                    app.world.get_resource_mut::<AudioState>()
                                {
                                    let volume = audio_state.get_volume();
                                    audio_state.set_volume((volume - 0.1).max(0.0));
                                }
                            }
                        }
                        KeyCode::Right => {
                            if app.menu.state == menu_types::MenuState::Options
                                && matches!(
                                    app.menu.options_selected,
                                    menu_types::OptionsOption::VolumeUp
                                )
                            {
                                // Volume up
                                if let Some(mut audio_state) =
                                    app.world.get_resource_mut::<AudioState>()
                                {
                                    let volume = audio_state.get_volume();
                                    audio_state.set_volume((volume + 0.1).min(1.0));
                                }
                            }
                        }
                        _ => {}
                    }

                    // Handle audio controls and then skip to next iteration
                    let mut input = app.world.resource_mut::<Input>();
                    match key.code {
                        KeyCode::Char('m') => input.toggle_music = true,
                        KeyCode::Char('+') | KeyCode::Char('=') => input.volume_up = true,
                        KeyCode::Char('-') | KeyCode::Char('_') => input.volume_down = true,
                        _ => {}
                    }

                    continue; // Skip the rest of the input processing for game controls
                }

                // Handle audio control keys regardless of game state
                let mut input = app.world.resource_mut::<Input>();
                match key.code {
                    KeyCode::Char('m') => input.toggle_music = true,
                    KeyCode::Char('+') | KeyCode::Char('=') => input.volume_up = true,
                    KeyCode::Char('-') | KeyCode::Char('_') => input.volume_down = true,
                    _ => {}
                }

                if is_game_over && key.code == KeyCode::Enter {
                    // First save the state we need to preserve
                    let was_hard_drop_released = input.hard_drop_released;

                    // Drop the current mutable borrow of input
                    drop(input);

                    // Reset game state
                    {
                        let mut game_state = app.world.resource_mut::<GameState>();
                        game_state.reset();
                    }

                    // Clear the board
                    {
                        let mut board = app.world.resource_mut::<Board>();
                        board.clear();
                    }

                    // Spawn new tetromino
                    systems::spawn_tetromino(&mut app.world);

                    // Re-acquire input after other operations and reset it
                    let mut input = app.world.resource_mut::<Input>();
                    *input = Input::default();
                    input.hard_drop_released = was_hard_drop_released;
                } else if !is_game_over {
                    // Update input state for normal gameplay
                    let mut input = app.world.resource_mut::<Input>();
                    match key.code {
                        KeyCode::Left | KeyCode::Char('a') => {
                            input.left = true;
                            input.right = false;
                        }
                        KeyCode::Right | KeyCode::Char('d') => {
                            input.right = true;
                            input.left = false;
                        }
                        KeyCode::Down | KeyCode::Char('s') => input.down = true,
                        KeyCode::Up | KeyCode::Char('w' | ' ') => {
                            input.rotate = true;
                        }
                        KeyCode::Enter => {
                            // Only set hard_drop to true if the key was previously released
                            if input.hard_drop_released {
                                input.hard_drop = true;
                                input.hard_drop_released = false; // Mark as not released until we see a release event
                            }
                        }
                        _ => (),
                    }
                }

                // Update last key in game state
                let mut game_state = app.world.resource_mut::<GameState>();
                game_state.last_key = Some(key);
            }
        }
    }
}
