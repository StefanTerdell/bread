use crate::tcp_message::TcpMessage;
use anyhow::{anyhow, bail, Result};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::str;
use std::sync::Arc;
use std::thread;

pub fn connect(stream: TcpStream) -> Result<()> {
    let remote = stream.peer_addr()?;
    let address = stream.local_addr()?;
    println!("✔ {address} <> {remote}");

    let stream = Arc::new(stream);

    let print_stream = stream.clone();
    let print_thread = thread::spawn(move || -> Result<()> {
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
        buf = buf.trim().to_string();

        let msg = if buf.is_empty() {
            TcpMessage::Nothing(address)
        } else if buf == ".exit" {
            TcpMessage::Leaving(address)
        } else {
            TcpMessage::Chat(address, buf.clone())
        };

        stream.as_ref().write_all(&msg.to_bytes()?)?;

        if msg.is_leaving() {
            break;
        }
    }

    print_thread.join().map_err(|err| anyhow!("{:?}", err))??;

    Ok(())
}

fn spawn_server(port: u16) -> Result<TcpStream> {
    let path = "C:\\Users\\stefan\\git\\rust\\bread\\target\\debug\\";
    let command =
        format!("{path}bread serve --port {port} --shutdown-after-last --silent --print-port");

    let mut child = Command::new("cmd")
        .args(["/C", &command])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or(anyhow!("Failed to grab stdout"))?;
    let mut buf = [0; 1000];

    loop {
        let bytes = stdout.read(&mut buf)?;
        let message = str::from_utf8(&buf[0..bytes])?;

        if message.starts_with(&port.to_string()) {
            break;
        } else if !message.is_empty() {
            bail!("Unknown message from server: {:?}", message);
        }
    }

    let address = format!("127.0.0.1:{}", port);
    println!("Spawned new server at {address}, connecting");

    Ok(TcpStream::connect(address)?)
}

pub fn connect_or_spawn_server(port: Option<u16>) -> Result<()> {
    let port = port.unwrap_or(3000);

    let stream =
        TcpStream::connect(format!("127.0.0.1:{}", port)).or_else(|_| spawn_server(port))?;

    connect(stream)
}
