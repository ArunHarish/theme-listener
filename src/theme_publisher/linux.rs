use super::ThemePublisher;

use std::error::Error;
use std::time::Duration;

// DBus
use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use dbus::{arg, Message};

use crate::theme::Theme;

struct OrgFreeDesktopPortalDesktop {
    pub sender: String,
    pub key: String,
    pub value: Variant<Box<dyn RefArg>>,
}
impl arg::AppendAll for OrgFreeDesktopPortalDesktop {
    fn append(&self, i: &mut arg::IterAppend) {
        RefArg::append(&self.sender, i);
    }
}

impl arg::ReadAll for OrgFreeDesktopPortalDesktop {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(OrgFreeDesktopPortalDesktop {
            sender: i.read()?,
            key: i.read()?,
            value: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for OrgFreeDesktopPortalDesktop {
    const NAME: &'static str = "SettingChanged";
    const INTERFACE: &'static str = "org.freedesktop.portal.Settings";
}

#[derive(Copy, Clone)]
pub struct DBusPublisher;

impl DBusPublisher {
    pub fn new() -> DBusPublisher {
        DBusPublisher {}
    }
}

impl ThemePublisher<i64> for DBusPublisher {
    fn fetch(self) -> Result<Theme, Box<dyn Error>> {
        let conn = Connection::new_session()?;
        let proxy = Proxy::new(
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            Duration::from_millis(5000),
            &conn,
        );
        let result: (Variant<Box<dyn RefArg>>,) = proxy.method_call(
            "org.freedesktop.portal.Settings",
            "Read",
            ("org.freedesktop.appearance", "color-scheme"),
        )?;
        let mut theme: Theme = Theme::DARK;

        if let Some(theme_value) = result.0 .0.as_i64() {
            theme = self.to_theme(theme_value);
        }
        Ok(theme)
    }

    fn on_publish(self, callback: Box<dyn Fn(Theme) + Send>) {
        let connection = Connection::new_session().unwrap();
        let proxy = connection.with_proxy(
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            Duration::from_millis(5000),
        );

        let _ = proxy.match_signal(
            move |h: OrgFreeDesktopPortalDesktop, _: &Connection, _: &Message| {
                if h.sender == "org.freedesktop.appearance" && h.key == "color-scheme" {
                    let next_theme_value = h.value.as_i64().unwrap();
                    let next_theme = self.to_theme(next_theme_value);
                    callback(next_theme);
                }
                true
            },
        );

        loop {
            connection.process(Duration::from_millis(1000)).unwrap();
        }
    }

    fn to_theme(self, value: i64) -> Theme {
        if value == 1 {
            return Theme::DARK;
        }
        return Theme::LIGHT;
    }
}
