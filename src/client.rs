use crate::tcp_message::{TcpMessage, TcpMessageError};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::str::{self, Utf8Error};
use std::sync::Arc;
use std::thread;

pub fn connect(stream: TcpStream) -> Result<(), ClientError> {
    println!("✔");
    let address = stream.local_addr()?;
    let stream = Arc::new(stream);

    let print_stream = stream.clone();
    let print_thread = thread::spawn(move || -> Result<(), ClientError> {
        let mut buf = [0u8; 1024];

        loop {
            let bytes = print_stream.as_ref().read(&mut buf)?;
            let msg = TcpMessage::from_bytes(&buf[0..bytes])?;

            if msg.get_address() == address {
                println!("** {:?}", msg);

                if msg.is_leaving() {
                    return Ok(());
                }
            } else {
                println!("-- {:?}", msg);
            }
        }
    });

    loop {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        buf.truncate(buf.len() - 2);

        let msg = if buf.len() == 0 {
            TcpMessage::Nothing(address)
        } else if buf == ".exit" {
            TcpMessage::Leaving(address)
        } else {
            TcpMessage::Chat(address, buf.clone())
        };

        stream.as_ref().write(&msg.to_bytes()?)?;

        if msg.is_leaving() {
            print_thread.join().expect("🦄")?;
            break;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum ClientError {
    Io(std::io::Error),
    Conversion(Utf8Error),
    StdoutGrabError,
    UnknownServerMessage(String),
    Message(TcpMessageError),
}

impl From<io::Error> for ClientError {
    fn from(err: io::Error) -> ClientError {
        return ClientError::Io(err);
    }
}

impl From<Utf8Error> for ClientError {
    fn from(err: Utf8Error) -> ClientError {
        return ClientError::Conversion(err);
    }
}

impl From<TcpMessageError> for ClientError {
    fn from(err: TcpMessageError) -> ClientError {
        return ClientError::Message(err);
    }
}

fn spawn_server(port: u16) -> Result<TcpStream, ClientError> {
    let path = "C:\\Users\\stefan\\git\\rust\\bread\\target\\debug\\";
    let command =
        format!("{path}bread serve --port {port} --shutdown-after-last --silent --print-port");

    let mut child = Command::new("cmd")
        .args(&["/C", &command])
        .stdout(Stdio::piped())
        // .stderr(Stdio::null())
        .spawn()?;

    // child.stderr.take();

    let mut stdout = child.stdout.take().ok_or(ClientError::StdoutGrabError)?;
    let mut buf = [0; 1000];

    loop {
        let bytes = stdout.read(&mut buf)?;
        let message = str::from_utf8(&buf[0..bytes])?;

        if message.starts_with(&port.to_string()) {
            break;
        } else if message != "" {
            return Err(ClientError::UnknownServerMessage(
                format!("Unknown message from server: {:?}", message).to_string(),
            ));
        }
    }

    let address = format!("127.0.0.1:{}", port);
    println!("Spawned new server at {address}, connecting");

    return Ok(TcpStream::connect(address)?);
}

pub fn connect_or_spawn_server(port: Option<u16>) -> Result<(), ClientError> {
    let port = port.unwrap_or(3000);

    let stream =
        TcpStream::connect(format!("127.0.0.1:{}", port)).or_else(|_| spawn_server(port))?;

    connect(stream)
}
