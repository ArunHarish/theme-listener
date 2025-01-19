pub mod theme {
    pub enum Theme {
        LIGHT,
        DARK,
    }
    pub fn to_theme(value: i64) -> Theme {
        if value == 1 {
            return Theme::DARK;
        }
        return Theme::LIGHT;
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
}
