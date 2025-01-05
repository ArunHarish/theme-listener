use std::error::Error;
use std::io::{self, BufWriter, Write};
use std::fmt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

// DBus
use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use dbus::{arg, Message};

// Threads and communication
use std::thread;
use std::sync::{Arc, Condvar, Mutex};

enum Theme {
    LIGHT,
    DARK,
}

pub struct OrgFreeDesktopPortalDesktop {
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

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

fn write_to_stream(stream: &mut BufWriter<UnixStream>, value: &[u8]) -> io::Result<()> {
    stream.write_all(value)?;
    stream.flush()
}

fn to_theme(value: i64) -> Theme {
    if value == 1 {
        return Theme::DARK;
    }
    return Theme::LIGHT;
}

fn detect_freedesktop_theme() -> Result<Theme, dbus::Error> {
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
        theme = to_theme(theme_value);
    }
    Ok(theme)
}

fn listen_freedesktop_theme(condvar_pair: Arc<(Mutex<Theme>, Condvar)>) -> Result<bool, dbus::Error> {
    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        Duration::from_millis(5000),
    );

    let _ = proxy.match_signal(move |h: OrgFreeDesktopPortalDesktop, _: &Connection, _: &Message| {
        if h.sender == "org.freedesktop.appearance" && h.key == "color-scheme" {
            let (mutex, condvar) = &*condvar_pair;
            let mut current_theme_value = mutex.lock().unwrap();
            let next_theme_value = h.value.as_i64().unwrap();
            *current_theme_value = to_theme(next_theme_value);
            condvar.notify_all();
        }
        true
    });

    loop {
        connection.process(Duration::from_millis(1000))?;
    }
}

fn handle_connect(socket_stream: UnixStream, condvar_pair: Arc<(Mutex<Theme>, Condvar)>) {
    // Handle stream here
    let mut stream = BufWriter::new(socket_stream);
    if let Ok(value) = detect_freedesktop_theme() {
        // Use stream to send theme value
        if let Err(_) = write_to_stream(&mut stream, value.to_string().as_bytes()) {
            return ();
        }
    } else {
        println!("WARNING: Error while fetching theme information");
    }

    let (mutex, condvar) = &*condvar_pair;
    let mut theme_value = mutex.lock().unwrap();
    loop {
       theme_value = condvar.wait(theme_value).unwrap();
       // On error stop block listen to theme value
       if let Err(_) = write_to_stream(&mut stream, theme_value.to_string().as_bytes()) {
           break;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Ok(listener) = UnixListener::bind("/tmp/theme-listener.sock") else {
        panic!("Address already in use");
    };
    let theme_condvar_main_pair = Arc::new((Mutex::new(Theme::DARK), Condvar::new()));
    let theme_condvar_publisher_pair = Arc::clone(&theme_condvar_main_pair);

    thread::spawn(move ||  listen_freedesktop_theme(theme_condvar_publisher_pair) );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let theme_condvar_subscriber_pair = Arc::clone(&theme_condvar_main_pair);
                thread::spawn(move || handle_connect(stream, theme_condvar_subscriber_pair));
            }
            Err(_) => {
                panic!("Stream error");
            }
        }
    }

    Ok(())
}
