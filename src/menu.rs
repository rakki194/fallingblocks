use crate::app::App;
use crate::components::{Particle, Position};
use crate::menu_types::{Menu, MenuOption, MenuState, OptionsOption};
use crate::particles;
use crate::sound::{AudioState, SoundEffect};
use bevy_ecs::prelude::*;
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
            title_colors: vec![Color::Red, Color::Yellow, Color::Green, Color::Blue],
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
                    app.reset();
                    true
                }
                MenuOption::Options => {
                    menu.state = MenuState::Options;
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
        // Update title colors
        if self.color_change_time.elapsed() > Duration::from_millis(100) {
            self.color_change_time = Instant::now();
            let first_color = self.title_colors.remove(0);
            self.title_colors.push(first_color);
        }

        // Spawn particles
        if self.last_particle_spawn.elapsed() > Duration::from_millis(50) {
            self.last_particle_spawn = Instant::now();
            if self.particles.len() < 100 {
                self.particles.push(Particle {
                    position: Position {
                        x: fastrand::i32(0..100),
                        y: fastrand::i32(0..100),
                    },
                    velocity: ((fastrand::f32() - 0.5) * 2.0, (fastrand::f32() - 0.5) * 2.0),
                    color: self.title_colors[fastrand::usize(0..self.title_colors.len())],
                    lifetime: fastrand::f32() * 1.2 + 0.5, // 0.5 to 1.7 seconds
                    size: fastrand::f32() * 0.8 + 0.2,     // 0.2 to 1.0
                });
            }
        }

        // No need to update particles, they don't have an update method
        // Just remove dead particles based on lifetime
        self.particles.retain(|p| p.lifetime > 0.0);
    }

    pub fn render_menu(f: &mut Frame, app: &App, menu: &Menu, renderer: &MenuRenderer) {
        let area = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        render_title(f, chunks[0], &renderer.title_colors);
        match menu.state {
            MenuState::MainMenu => render_main_menu_options(f, chunks[1], menu),
            MenuState::Options => render_options_menu(f, chunks[1], menu, app),
            MenuState::Game => {}
        }
        render_menu_particles(f, renderer);
    }
}

fn render_title(f: &mut Frame, area: Rect, colors: &[Color]) {
    let title = "FALLING BLOCKS";
    let title_style = Style::default().fg(colors[0]).add_modifier(Modifier::BOLD);
    let title_line = Line::from(Span::styled(title, title_style));
    let title_block = Block::default().borders(Borders::ALL).title(title_line);
    f.render_widget(title_block, area);
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
