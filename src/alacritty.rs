use serde::Deserialize;
use std::error::Error;
use std::fs::read_to_string;
use std::io::Write;
use std::os::unix::net::UnixStream;

use std::path::Path;
use toml::Table;

use crate::theme::{Theme, ThemeListener};

#[derive(Deserialize, Debug)]
struct AlacrittyTheme {
    colors: Table,
}

fn flatten_table(config: Table) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result: Vec<String> = vec![];
    let mut keys: Vec<String> = vec!["colors".to_string()];
    let mut config_stack: Vec<&Table> = vec![&config];

    while let Some(config_value) = config_stack.pop() {
        let current_prefix = keys.pop().ok_or("No prefix available")?;
        for entries in config_value {
            let (key, value) = entries;
            // Append it to the stack if value is a table
            if let Some(value) = value.as_table() {
                keys.push(format!("{}.{}", current_prefix, key));
                config_stack.push(value);
            } else if let Some(value) = value.as_str() {
                result.push(format!("\"{}.{}=\\\"{}\\\"\"", current_prefix, key, value));
            } else if let Some(value_list) = value.as_array() {
                value_list.iter().enumerate().for_each(|(i, value)| {
                    let index = i.to_string();
                    keys.push(format!("{}.{}.[{}]", current_prefix, key, index));
                    if let Some(value) = value.as_table() {
                        config_stack.push(value);
                    }
                });
            }
        }
    }

    return Ok(result);
}

#[derive(Clone)]
pub struct Alacritty {
    socket_path: String,
    light_theme_config_path: String,
    dark_theme_config_path: String,
}

impl Alacritty {
    pub fn new() -> Alacritty {
        let socket_env = std::env::var("ALACRITTY_SOCKET").unwrap_or(String::from(""));

        let home_directory_env = std::env::var("HOME").unwrap_or(String::from(""));
        let home_directory_path = Path::new(&home_directory_env);

        let alacritty_config_directory = home_directory_path.join(".config/alacritty/themes/");

        let light_theme =
            std::env::var("ALACRITTY_LIGHT_THEME").unwrap_or(String::from("Light theme not set"));
        let dark_theme =
            std::env::var("ALACRITTY_DARK_THEME").unwrap_or(String::from("Dark theme not set"));

        if light_theme.is_empty() || dark_theme.is_empty() {
            panic!("Alacritty theme path environment variables not found");
        }

        let light_theme_config_path =
            alacritty_config_directory.join(format!("{light_theme}.toml"));
        let dark_theme_config_path = alacritty_config_directory.join(format!("{dark_theme}.toml"));

        if !light_theme_config_path.exists() || !dark_theme_config_path.exists() {
            panic!("Alacritty config paths not found");
        }

        Alacritty {
            socket_path: socket_env,
            light_theme_config_path: light_theme_config_path.to_str().unwrap().to_string(),
            dark_theme_config_path: dark_theme_config_path.to_str().unwrap().to_string(),
        }
    }
}

impl ThemeListener<usize> for Alacritty {
    fn handle(self, next_theme_value: Theme) -> std::io::Result<usize> {
        let theme: AlacrittyTheme;
        match next_theme_value {
            Theme::DARK => {
                let dark_theme_config = read_to_string(self.dark_theme_config_path)?;
                theme = toml::from_str(&dark_theme_config).unwrap();
            }
            Theme::LIGHT => {
                let light_theme_config = read_to_string(self.light_theme_config_path)?;
                theme = toml::from_str(&light_theme_config).unwrap();
            }
        }
        let options = flatten_table(theme.colors).unwrap().join(",");
        let value = format!(
            r#"{{"Config":{{"options": [{}],"reset": false}}}}"#,
            options
        );

        // Write JSON
        let mut connection = UnixStream::connect(self.socket_path.clone())?;
        connection.write(value.as_bytes())
    }
}
