// Theme module
mod theme;

// Client modules
mod alacritty;
mod tmux;

use std::error::Error;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};

// Threads and communication
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// To check whether socket exists
use std::fs::{exists, remove_file};

// For libc signal handling
use libc::{fork, sigaction, SA_SIGINFO, SIGHUP, SIGINT, SIGTERM};
use std::mem;
use std::process::exit;
use std::ptr;

// Theme import
use crate::theme::{Theme, ThemeListener, ThemePublisher};

// Alacritty import
use crate::alacritty::Alacritty;

// Tmux import
use crate::tmux::Tmux;

// Linux
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use crate::linux::DBusPublisher;
#[cfg(target_os = "linux")]
fn publisher_factory() -> DBusPublisher {
    return DBusPublisher::new();
}

// Mac os
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use crate::macos::KVOPublisher;
#[cfg(target_os = "macos")]
fn publisher_factory() -> KVOPublisher {
    return KVOPublisher::new();
}

const SOCKET_PATH: &str = "/tmp/theme-listener.sock";

fn write_to_stream(stream: &mut BufWriter<UnixStream>, value: String) -> io::Result<()> {
    stream.write_all(format!("{value}\n").as_bytes())?;
    stream.flush()
}

fn handle_stream<A, B>(listener: A)
where
    A: ThemeListener<B> + Clone,
{
    let Ok(theme_stream) = UnixStream::connect(SOCKET_PATH) else {
        panic!("Error listening to server");
    };

    let mut reader = BufReader::new(theme_stream);

    loop {
        let mut content = String::new();
        if let Ok(_) = reader.read_line(&mut content) {
            let theme_value = theme::to_theme(content.trim());
            listener.clone().handle(theme_value).unwrap();
        }
    }
}

fn listen_theme<A, B>(publisher: A, condvar_pair: Arc<(Mutex<Theme>, Condvar)>)
where
    A: ThemePublisher<B>,
{
    publisher.on_publish(Box::new(move |value: Theme| {
        let (mutex, condvar) = &*condvar_pair;
        let mut current_theme_value = mutex.lock().unwrap();
        *current_theme_value = value;
        condvar.notify_all();
        ()
    }));
}

fn handle_connect<A, B>(
    publisher: A,
    condvar_pair: Arc<(Mutex<Theme>, Condvar)>,
    socket_stream: UnixStream,
) where
    A: ThemePublisher<B>,
{
    // Handle stream here
    let mut stream = BufWriter::new(socket_stream);
    if let Ok(value) = publisher.fetch() {
        // Use stream to send theme value
        if let Err(_) = write_to_stream(&mut stream, value.to_string()) {
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
        if let Err(_) = write_to_stream(&mut stream, theme_value.to_string()) {
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
    unsafe {
        if std::env::args().any(|args| args == "-init") {
            if exists(SOCKET_PATH)? {
                return Ok(());
            }

            // Start the UNIX socket server
            let Ok(listener) = UnixListener::bind(SOCKET_PATH) else {
                panic!("Error while starting the listener server");
            };

            let process_id = fork();
            // If parent exit
            if process_id != 0 {
                return Ok(());
            }

            // On exit delete the socket file
            let action = sigaction {
                sa_sigaction: handle_terminate as usize,
                sa_flags: SA_SIGINFO,
                // sa_restorer: None,
                sa_mask: mem::zeroed(),
            };
            sigaction(SIGINT, &action, ptr::null_mut());
            sigaction(SIGTERM, &action, ptr::null_mut());
            sigaction(SIGHUP, &action, ptr::null_mut());

            let publisher = publisher_factory();
            let theme_condvar_main_pair =
                Arc::new((Mutex::new(publisher.fetch().unwrap()), Condvar::new()));
            let theme_condvar_pub_pair = Arc::clone(&theme_condvar_main_pair);
            let theme_condvar_sub_pair = Arc::clone(&theme_condvar_main_pair);

            // Listening to incoming connections
            thread::spawn(move || {
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            let theme_client_subscriber = theme_condvar_sub_pair.clone();
                            thread::spawn(move || {
                                handle_connect(publisher, theme_client_subscriber, stream);
                            });
                        }
                        Err(_) => {
                            panic!("Stream error");
                        }
                    }
                }
            });

            listen_theme(publisher, theme_condvar_pub_pair);
            return Ok(());
        } else if std::env::args().any(|args| args == "-d") {
            let process_id = fork();
            // If parent process then terminate
            if process_id != 0 {
                return Ok(());
            }
        }

        if std::env::args().any(|args| args == "-alacritty") {
            let alacritty = Alacritty::new();
            handle_stream(alacritty);
        } else if std::env::args().any(|args| args == "-tmux") {
            let tmux = Tmux::new();
            handle_stream(tmux);
        }

        Ok(())
    }
}
