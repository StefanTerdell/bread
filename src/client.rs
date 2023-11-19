use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::str::{self, Utf8Error};
use std::sync::Arc;
use std::thread;

use crate::tcp_message::TcpMessage;

pub fn connect(stream: TcpStream) -> Result<(), ClientError> {
    let stream = Arc::new(stream);

    let read_stream = stream.clone();

    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match read_stream.as_ref().read(&mut buf) {
                Ok(bytes) => {
                    let msg = TcpMessage::from_bytes(&buf[0..bytes]);

                    println!("{:?}", msg);
                }
                Err(err) => {
                    println!("Unable to read from stream. Breaking read-loop. {}", err);
                    break;
                }
            }
        }
    });

    loop {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        buf.truncate(buf.len() - 2);

        let msg = if buf.len() == 0 {
            TcpMessage::Nothing
        } else if buf == ".exit" {
            TcpMessage::Leaving
        } else {
            TcpMessage::Chat(buf.clone())
        };

        if let Err(_) = stream.as_ref().write(&msg.to_bytes()) {}

        if msg == TcpMessage::Leaving {
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

fn spawn_server(port: u16) -> Result<TcpStream, ClientError> {
    let path = "C:\\Users\\stefan\\git\\rust\\bread\\target\\debug\\";
    let command = format!("{path}bread.exe serve --port {port} --shutdown-after-last --silent");
    let mut child = Command::new("cmd")
        .args(&["/C", &command])
        .stdout(Stdio::piped())
        .spawn()?;

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

    return Ok(TcpStream::connect(format!("127.0.0.1:{}", port))?);
}

pub fn connect_or_spawn_server(port: Option<u16>) -> Result<(), ClientError> {
    let port = port.unwrap_or(3000);

    let stream =
        TcpStream::connect(format!("127.0.0.1:{}", port)).or_else(|_| spawn_server(port))?;

    connect(stream)
}
