use std::os::unix::net::{ UnixListener, UnixStream } ;
use std::io::{ BufWriter, Write } ;
use std::thread;

fn handle_connect(socket_stream: UnixStream) {
    // Handle stream here
    let mut stream =  BufWriter::new(socket_stream);
    stream.write(b"COOL").unwrap();

    stream.flush().unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = UnixListener::bind("/tmp/theme-listener.sock")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_connect(stream));
            },
            Err(_) => {
                panic!("Stream error");
            }
        }
    }

    Ok(())
}
