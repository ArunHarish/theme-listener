use crate::theme::Theme;

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

// Module exports
pub mod alacritty;
pub mod tmux;
