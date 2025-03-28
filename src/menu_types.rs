// Game state enum for main menu
#[derive(Clone, PartialEq)]
pub enum MenuState {
    MainMenu,
    Options,
    Game,
}

// Menu option selection
#[derive(Clone)]
pub enum MenuOption {
    NewGame,
    Options,
    Quit,
}

#[derive(Clone)]
pub enum OptionsOption {
    MusicToggle,
    SoundToggle,
    VolumeUp,
    VolumeDown,
    GridToggle,
    Back,
}

#[derive(Clone)]
pub struct Menu {
    pub state: MenuState,
    pub selected_option: MenuOption,
    pub options_selected: OptionsOption,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            state: MenuState::MainMenu,
            selected_option: MenuOption::NewGame,
            options_selected: OptionsOption::Back,
        }
    }
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }
}
