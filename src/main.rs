#![warn(clippy::all, clippy::pedantic)]

mod app;
mod components;
mod game;
mod particles;
mod systems;
mod ui;

use std::io;
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

fn main() -> AppResult<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
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
                    if key.code == KeyCode::Enter { input.hard_drop_released = true }
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

                if is_game_over && key.code == KeyCode::Enter {
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

                    // Clear input state after spawning new tetromino to prevent any unwanted actions
                    // while preserving the hard_drop_released flag
                    let mut input = app.world.resource_mut::<Input>();
                    let was_hard_drop_released = input.hard_drop_released;
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

        // Update game logic at a fixed rate
        if last_game_tick.elapsed() >= game_tick_rate {
            // First update time and get delta
            let delta_seconds = {
                let mut time = app.world.resource_mut::<Time>();
                time.update();
                time.delta_seconds()
            };

            debug!("Game tick at time: {:?}", Instant::now());

            // Process input first
            systems::input_system(&mut app.world);

            // Then update game state
            systems::game_tick_system(&mut app.world, delta_seconds);

            // Reset input state after processing
            let mut input = app.world.resource_mut::<Input>();
            *input = Input::default();

            last_game_tick = Instant::now();
        }

        if app.should_quit {
            info!("Game quit by user");
            return Ok(());
        }
    }
}
