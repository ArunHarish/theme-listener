use crate::theme::Theme;
use std::error::Error;

cfg_if::cfg_if!(
    if #[cfg(target_os = "linux")] {
        mod linux;
        use crate::theme_publisher::linux::DBusPublisher;
        pub fn create_publisher() -> DBusPublisher {
            DBusPublisher::new()
        }
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        use crate::theme_publisher::macos::KVOPublisher;
        pub fn create_publisher() -> KVOPublisher {
            KVOPublisher::new()
        }
    }
);

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
