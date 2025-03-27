use crate::app::App;
use crate::components::{Particle, Position};
use crate::menu_types::{Menu, MenuOption, MenuState, OptionsOption};
use crate::particles;
use crate::sound::{AudioState, SoundEffect};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::time::Duration;
use std::time::Instant;

// ASCII art letters for "FALLINGBLOCKS" title
const TITLE_LETTERS: [&str; 13] = [
    // F
    "████\n█   \n███ \n█   \n█   ",
    // A
    " ██ \n█  █\n████\n█  █\n█  █",
    // L
    "█   \n█   \n█   \n█   \n████",
    // L
    "█   \n█   \n█   \n█   \n████",
    // I
    "███\n █ \n █ \n █ \n███",
    // N
    "█  █\n██ █\n█ ██\n█  █\n█  █",
    // G
    " ███ \n█    \n█  ██\n█   █\n ███ ",
    // B
    "███ \n█  █\n███ \n█  █\n███ ",
    // L
    "█   \n█   \n█   \n█   \n████",
    // O
    " ██ \n█  █\n█  █\n█  █\n ██ ",
    // C
    " ███\n█   \n█   \n█   \n ███",
    // K
    "█  █\n█ █ \n██  \n█ █ \n█  █",
    // S
    " ███\n█   \n ██ \n   █\n███ ",
];

pub struct MenuRenderer {
    pub particles: Vec<Particle>,
    pub last_particle_spawn: Instant,
    pub title_colors: Vec<Color>,
    pub color_change_time: Instant,
}

impl Default for MenuRenderer {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            last_particle_spawn: Instant::now(),
            title_colors: vec![
                Color::Red,
                Color::Yellow,
                Color::Green,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
            ],
            color_change_time: Instant::now(),
        }
    }
}

impl MenuRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_option(&mut self, menu: &mut Menu) {
        match menu.state {
            MenuState::MainMenu => {
                menu.selected_option = match menu.selected_option {
                    MenuOption::NewGame => MenuOption::Options,
                    MenuOption::Options => MenuOption::Quit,
                    MenuOption::Quit => MenuOption::NewGame,
                };
            }
            MenuState::Options => {
                menu.options_selected = match menu.options_selected {
                    OptionsOption::MusicToggle => OptionsOption::SoundToggle,
                    OptionsOption::SoundToggle => OptionsOption::VolumeUp,
                    OptionsOption::VolumeUp => OptionsOption::VolumeDown,
                    OptionsOption::VolumeDown => OptionsOption::Back,
                    OptionsOption::Back => OptionsOption::MusicToggle,
                };
            }
            MenuState::Game => {}
        }
    }

    pub fn prev_option(&mut self, menu: &mut Menu) {
        match menu.state {
            MenuState::MainMenu => {
                menu.selected_option = match menu.selected_option {
                    MenuOption::NewGame => MenuOption::Quit,
                    MenuOption::Options => MenuOption::NewGame,
                    MenuOption::Quit => MenuOption::Options,
                };
            }
            MenuState::Options => {
                menu.options_selected = match menu.options_selected {
                    OptionsOption::MusicToggle => OptionsOption::Back,
                    OptionsOption::SoundToggle => OptionsOption::MusicToggle,
                    OptionsOption::VolumeUp => OptionsOption::SoundToggle,
                    OptionsOption::VolumeDown => OptionsOption::VolumeUp,
                    OptionsOption::Back => OptionsOption::VolumeDown,
                };
            }
            MenuState::Game => {}
        }
    }

    pub fn select(&mut self, menu: &mut Menu, app: &mut App) -> bool {
        match menu.state {
            MenuState::MainMenu => match menu.selected_option {
                MenuOption::NewGame => {
                    menu.state = MenuState::Game;
                    // Play a sound effect when starting the game
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        if audio_state.is_sound_enabled() {
                            audio_state.play_sound(SoundEffect::LevelUp);
                        }
                    }
                    app.reset();
                    true
                }
                MenuOption::Options => {
                    menu.state = MenuState::Options;
                    // Play menu navigation sound
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        if audio_state.is_sound_enabled() {
                            audio_state.play_sound(SoundEffect::Move);
                        }
                    }
                    true
                }
                MenuOption::Quit => true,
            },
            MenuState::Options => match menu.options_selected {
                OptionsOption::MusicToggle => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        audio_state.toggle_music();
                    }
                    true
                }
                OptionsOption::SoundToggle => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        // Toggle sound enabled state
                        audio_state.toggle_sound();
                    }
                    true
                }
                OptionsOption::VolumeUp => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        let volume = audio_state.get_volume();
                        audio_state.set_volume((volume + 0.1).min(1.0));
                    }
                    true
                }
                OptionsOption::VolumeDown => {
                    if let Some(mut audio_state) = app.world.get_resource_mut::<AudioState>() {
                        let volume = audio_state.get_volume();
                        audio_state.set_volume((volume - 0.1).max(0.0));
                    }
                    true
                }
                OptionsOption::Back => {
                    menu.state = MenuState::MainMenu;
                    true
                }
            },
            MenuState::Game => false,
        }
    }

    pub fn update(&mut self) {
        // Update title colors - rotate colors for animation effect
        if self.color_change_time.elapsed() > Duration::from_millis(80) {
            self.color_change_time = Instant::now();
            let first_color = self.title_colors.remove(0);
            self.title_colors.push(first_color);
        }

        // Spawn particles
        if self.last_particle_spawn.elapsed() > Duration::from_millis(20) {
            self.last_particle_spawn = Instant::now();

            // Calculate the approximate title area
            // The title starts around 1/5 of the way down from the top
            // and extends across most of the width
            let title_start_y = 5;
            let title_height = 5; // ASCII art height
            let title_width_percent = 80; // Title covers about 80% of screen width

            if self.particles.len() < 200 {
                // Create particles around and beneath the title
                let spawn_mode = fastrand::usize(0..10);

                if spawn_mode < 5 {
                    // Spawn particles from the title letters
                    let x = fastrand::i32(10..title_width_percent);
                    let y = title_start_y + fastrand::i32(0..title_height);

                    self.particles.push(Particle {
                        position: Position { x, y },
                        velocity: (
                            (fastrand::f32() - 0.5) * 1.5, // Left/right drift
                            fastrand::f32() * 1.5 + 0.5,   // Falling down
                        ),
                        color: self.title_colors[fastrand::usize(0..self.title_colors.len())],
                        lifetime: fastrand::f32() * 1.5 + 0.8, // 0.8 to 2.3 seconds
                        size: fastrand::f32() * 0.8 + 0.2,     // 0.2 to 1.0
                    });
                } else {
                    // Occasionally spawn particles from top of screen
                    let x = fastrand::i32(5..95);

                    self.particles.push(Particle {
                        position: Position {
                            x,
                            y: fastrand::i32(0..3),
                        },
                        velocity: (
                            (fastrand::f32() - 0.5) * 0.5, // Slight horizontal drift
                            fastrand::f32() * 2.0 + 1.0,   // Falling down faster
                        ),
                        color: self.title_colors[fastrand::usize(0..self.title_colors.len())],
                        lifetime: fastrand::f32() * 2.0 + 1.0, // 1.0 to 3.0 seconds
                        size: fastrand::f32() * 0.8 + 0.2,     // 0.2 to 1.0
                    });
                }
            }
        }

        // Update existing particles
        for particle in &mut self.particles {
            // Update position based on velocity
            particle.position.x = (particle.position.x as f32 + particle.velocity.0) as i32;
            particle.position.y = (particle.position.y as f32 + particle.velocity.1) as i32;

            // Reduce lifetime
            particle.lifetime -= 0.016; // Assuming ~60fps
        }

        // Remove expired particles
        self.particles
            .retain(|p| p.lifetime > 0.0 && p.position.y < 100);
    }

    pub fn render_menu(f: &mut Frame, app: &App, menu: &Menu, renderer: &MenuRenderer) {
        let area = f.area();

        // Draw particles first so they appear behind the text
        render_menu_particles(f, renderer);

        // Create a better layout with more space for the ASCII title
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Space for title
                Constraint::Min(3),     // Space for menu options
                Constraint::Length(1),  // Bottom padding
            ])
            .split(area);

        render_title(f, chunks[0], &renderer.title_colors);
        match menu.state {
            MenuState::MainMenu => render_main_menu_options(f, chunks[1], menu),
            MenuState::Options => render_options_menu(f, chunks[1], menu, app),
            MenuState::Game => {}
        }
    }
}

fn render_title(f: &mut Frame, area: Rect, colors: &[Color]) {
    // Split title area into multiple chunks to fit the ASCII art
    let letter_width = 6; // Maximum width of our ASCII art letters + spacing

    let total_width = TITLE_LETTERS.len() as u16 * letter_width;
    let start_x = area.width.saturating_sub(total_width) / 2;

    // Render each ASCII art letter with its own color
    for (i, letter) in TITLE_LETTERS.iter().enumerate() {
        let color_idx = (i + colors.len() - (i % colors.len())) % colors.len();
        let letter_style = Style::default().fg(colors[color_idx]);

        let letter_lines: Vec<&str> = letter.lines().collect();
        let letter_height = letter_lines.len() as u16;

        let letter_x = start_x + (i as u16 * letter_width);
        let letter_y = area.y + (area.height.saturating_sub(letter_height) / 2);

        for (y_offset, line) in letter_lines.iter().enumerate() {
            let rect = Rect::new(letter_x, letter_y + y_offset as u16, line.len() as u16, 1);

            if rect.x < area.width && rect.y < area.height {
                let text = Text::from(Line::from(Span::styled(*line, letter_style)));
                let paragraph = Paragraph::new(text);
                f.render_widget(paragraph, rect);
            }
        }
    }
}

fn render_main_menu_options(f: &mut Frame, area: Rect, menu: &Menu) {
    let options = vec!["New Game", "Options", "Quit"];
    let mut lines = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let style = if i
            == match menu.selected_option {
                MenuOption::NewGame => 0,
                MenuOption::Options => 1,
                MenuOption::Quit => 2,
            } {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![Span::styled(option.to_string(), style)]));
    }
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_options_menu(f: &mut Frame, area: Rect, menu: &Menu, app: &App) {
    let mut options = Vec::new();

    // Get audio state from the world
    if let Some(audio_state) = app.world.get_resource::<AudioState>() {
        options.push(format!(
            "Music: {}",
            if audio_state.is_music_enabled() {
                "ON"
            } else {
                "OFF"
            }
        ));

        options.push(format!(
            "Sound: {}",
            if audio_state.is_sound_enabled() {
                "ON"
            } else {
                "OFF"
            }
        ));

        options.push(format!("Volume: {:.1}", audio_state.get_volume()));
    } else {
        // Fallback if audio state isn't available
        options.push("Music: N/A".to_string());
        options.push("Sound: N/A".to_string());
        options.push("Volume: N/A".to_string());
    }

    options.push("Back".to_string());

    let mut lines = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let style = if i
            == match menu.options_selected {
                OptionsOption::MusicToggle => 0,
                OptionsOption::SoundToggle => 1,
                OptionsOption::VolumeUp => 2,
                OptionsOption::VolumeDown => 2,
                OptionsOption::Back => 3,
            } {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![Span::styled(option.to_string(), style)]));
    }
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_menu_particles(f: &mut Frame, renderer: &MenuRenderer) {
    for particle in &renderer.particles {
        let x = particle.position.x as u16;
        let y = particle.position.y as u16;

        // Skip if particle is out of bounds
        if x >= f.area().width || y >= f.area().height {
            continue;
        }

        let area = Rect::new(x, y, 1, 1);
        let block = Block::default().style(Style::default().fg(particle.color));
        f.render_widget(block, area);
    }
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;

    Rect::new(x, y, width.min(r.width), height.min(r.height))
}
