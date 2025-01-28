use std::io::Result;
use std::path::Path;
use std::process::{Command, Output};

use crate::theme::{Theme, ThemeListener};

#[derive(Clone)]
pub struct Tmux {
    light_theme_config_path: String,
    dark_theme_config_path: String,
}

impl Tmux {
    pub fn new() -> Tmux {
        let home_directory_env = std::env::var("HOME").unwrap_or(String::from(""));
        let home_directory_path = Path::new(&home_directory_env);
        let tmux_config_directory = home_directory_path.join(".config/tmux/themes/");
        if !home_directory_path.exists() {
            panic!("Home directory path is invalid");
        }

        if !tmux_config_directory.exists() {
            panic!("TMUX config directory path not found");
        }

        let light_theme =
            std::env::var("TMUX_LIGHT_THEME").unwrap_or(String::from("Light theme not set"));
        let dark_theme =
            std::env::var("TMUX_DARK_THEME").unwrap_or(String::from("Dark theme not set"));

        let light_theme_config_path = tmux_config_directory.join(format!("{light_theme}.config"));
        let dark_theme_config_path = tmux_config_directory.join(format!("{dark_theme}.config"));

        if !light_theme_config_path.exists() || !dark_theme_config_path.exists() {
            panic!("Config paths not found");
        }

        Tmux {
            light_theme_config_path: light_theme_config_path.to_str().unwrap().to_string(),
            dark_theme_config_path: dark_theme_config_path.to_str().unwrap().to_string(),
        }
    }
}

impl ThemeListener<Output> for Tmux {
    fn handle(self, next_theme_value: Theme) -> Result<Output> {
        let selected_theme: &str;
        match next_theme_value {
            Theme::DARK => {
                selected_theme = &self.dark_theme_config_path;
            }
            Theme::LIGHT => {
                selected_theme = &self.light_theme_config_path;
            }
        }
        Command::new("tmux")
            .args(["source", selected_theme])
            .output()
    }
}
