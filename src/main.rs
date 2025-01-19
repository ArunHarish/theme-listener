// Modules
mod theme;

use std::error::Error;
use std::io::{self, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

// DBus
use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use dbus::{arg, Message};

// Threads and communication
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// To check whether socket exists
use std::fs::{exists, remove_file};

// For libc signal handling
use libc::{fork, sigaction, SA_SIGINFO, SIGINT, SIGTERM};
use std::mem;
use std::process::exit;
use std::ptr;

// Theme import
use crate::theme::{to_theme, Theme};

const SOCKET_PATH: &str = "/tmp/theme-listener.sock";

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

fn write_to_stream(stream: &mut BufWriter<UnixStream>, value: &[u8]) -> io::Result<()> {
    stream.write_all(value)?;
    stream.flush()
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

fn listen_freedesktop_theme(
    condvar_pair: Arc<(Mutex<Theme>, Condvar)>,
) -> Result<bool, dbus::Error> {
    let connection = Connection::new_session()?;
    let proxy = connection.with_proxy(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        Duration::from_millis(5000),
    );

    let _ = proxy.match_signal(
        move |h: OrgFreeDesktopPortalDesktop, _: &Connection, _: &Message| {
            if h.sender == "org.freedesktop.appearance" && h.key == "color-scheme" {
                let (mutex, condvar) = &*condvar_pair;
                let mut current_theme_value = mutex.lock().unwrap();
                let next_theme_value = h.value.as_i64().unwrap();
                *current_theme_value = to_theme(next_theme_value);
                condvar.notify_all();
            }
            true
        },
    );

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

extern "C" fn handle_terminate() {
    // Delete the socket file
    match remove_file(SOCKET_PATH) {
        Ok(_) => {}
        Err(_) => println!("Error removing socket file"),
    }
    exit(0);
}

fn main() -> Result<(), Box<dyn Error>> {
    // Daemonise the process if -d flag is present
    if std::env::args().find(|args| args == "-d") != None {
        unsafe {
            let process_id = fork();
            // If parent process then terminate
            if process_id != 0 {
                return Ok(());
            }
        }
    }

    if exists(SOCKET_PATH)? {
        return Ok(());
    }

    // On exit delete the socket file
    unsafe {
        let action = sigaction {
            sa_sigaction: handle_terminate as usize,
            sa_flags: SA_SIGINFO,
            sa_restorer: None,
            sa_mask: mem::zeroed(),
        };
        sigaction(SIGINT, &action, ptr::null_mut());
        sigaction(SIGTERM, &action, ptr::null_mut());
    }

    let Ok(listener) = UnixListener::bind(SOCKET_PATH) else {
        panic!("Address already in use");
    };
    let theme_condvar_main_pair = Arc::new((Mutex::new(Theme::DARK), Condvar::new()));
    let theme_condvar_publisher_pair = Arc::clone(&theme_condvar_main_pair);

    thread::spawn(move || listen_freedesktop_theme(theme_condvar_publisher_pair));

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
