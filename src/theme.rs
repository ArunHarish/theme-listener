use std::error::Error;

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

/**
 * Trait for listeners to implement their own custom logic.
 */
pub trait ThemeListener<T> {
    /**
     *
     * @param next_theme_value Gets the applied theme value.
     * @return The IO call result of the listener.
     */
    fn handle(self, next_theme_value: Theme) -> std::io::Result<T>;
}

/**
 * Trait to implement logic to convert publisher specific value to the common
 * Theme value defined in this file.
 */
pub trait ThemePublisher<T> {
    /**
     * Fetches the current theme value
     */
    fn fetch(self) -> Result<Theme, Box<dyn Error>>;

    /**
     * A function to trigger on theme change
     * @param callback function to be called on theme change
     */
    fn on_publish(self, callback: Box<dyn Fn(Theme) + Send>);

    /**
     * Method to convert publisher value to Theme
     * @param value The publisher specific input
     */
    fn to_theme(self, value: T) -> Theme;
}
