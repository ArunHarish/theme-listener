#[derive(Clone, Copy)]
pub enum Theme {
    LIGHT,
    DARK,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::LIGHT => {
                write!(f, "light")
            }
            Theme::DARK => {
                write!(f, "dark")
            }
        }
    }
}

/**
 * To convert a string representation of theme value to Theme
 */
pub fn to_theme(value: &str) -> Theme {
    match value {
        "light" => Theme::LIGHT,
        "dark" => Theme::DARK,
        _ => panic!("Invalid theme value to parse"),
    }
}
