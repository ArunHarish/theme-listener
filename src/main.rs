// Theme module
mod theme;

// Linux
mod linux;

// Client modules
mod alacritty;
mod tmux;

use std::error::Error;
use std::io::{self, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};

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
use crate::theme::{Theme, ThemeListener, ThemePublisher};

// Alacritty import
use crate::alacritty::Alacritty;

// Tmux import
use crate::tmux::Tmux;

// Linux import
use crate::linux::DBusPublisher;

const SOCKET_PATH: &str = "/tmp/theme-listener.sock";

fn write_to_stream(stream: &mut BufWriter<UnixStream>, value: &[u8]) -> io::Result<()> {
    stream.write_all(value)?;
    stream.flush()
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

    let alacritty = Alacritty::new();
    let tmux = Tmux::new();
    let publisher = DBusPublisher::new();

    let theme_condvar_main_pair =
        Arc::new((Mutex::new(publisher.fetch().unwrap()), Condvar::new()));
    let theme_condvar_pub_pair = Arc::clone(&theme_condvar_main_pair);
    let theme_condvar_sub_pair = Arc::clone(&theme_condvar_main_pair);

    thread::spawn(move || listen_theme(publisher, theme_condvar_pub_pair));

    // For alacritty and tmux
    let theme_alacritty_tmux_sub_pair = theme_condvar_sub_pair.clone();
    thread::spawn(move || {
        let (mutex, condvar) = &*theme_alacritty_tmux_sub_pair;
        let mut theme_value = mutex.lock().unwrap();
        // Initial call
        alacritty.clone().handle(*theme_value).unwrap();
        tmux.clone().handle(*theme_value).unwrap();
        loop {
            theme_value = condvar.wait(theme_value).unwrap();
            alacritty.clone().handle(*theme_value).unwrap();
            tmux.clone().handle(*theme_value).unwrap();
        }
    });

    // For neovim
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let theme_nvim_client_subscriber_pair = theme_condvar_sub_pair.clone();
                thread::spawn(move || {
                    handle_connect(publisher, theme_nvim_client_subscriber_pair, stream)
                });
            }
            Err(_) => {
                panic!("Stream error");
            }
        }
    }

    Ok(())
}
