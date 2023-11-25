use crate::TcpMessage;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug)]
pub enum MpscMessage {
    Connect(SocketAddr, Arc<TcpStream>),
    Message(SocketAddr, TcpMessage),
    Disconnect(SocketAddr),
    Error(SocketAddr, String),
}

#[derive(Debug)]
struct Connection {
    stream: Arc<TcpStream>,
}

fn host_connections(
    mpsc_receiver: Receiver<MpscMessage>,
    shutdown_after_last: bool,
    silent: bool,
) -> Result<()> {
    let mut connections = HashMap::new();

    loop {
        match mpsc_receiver.recv()? {
            MpscMessage::Connect(address, stream) => {
                connections.insert(address, Connection { stream });

                if !silent {
                    println!("Client connected. Current count: {}", connections.len());
                }
            }
            MpscMessage::Message(_, msg) => {
                let msg = msg.to_bytes()?;

                for connection in connections.values() {
                    connection.stream.as_ref().write_all(&msg)?;
                }
            }
            MpscMessage::Error(address, error) => {
                connections.remove(&address);

                if !silent {
                    eprintln!(
                        "Client disconnected due to error. Current count: {}. Error: {:?}",
                        connections.len(),
                        error
                    );
                }

                if connections.is_empty() && shutdown_after_last {
                    if !silent {
                        println!("No connections left! Shutting down ✌");
                    }
                    break;
                }
            }
            MpscMessage::Disconnect(address) => {
                connections.remove(&address);

                if !silent {
                    println!("Client disconnected. Current count: {}", connections.len());
                }

                if connections.is_empty() && shutdown_after_last {
                    if !silent {
                        println!("No connections left! Shutting down ✌");
                    }
                    break;
                }
            }
        };
    }

    Ok(())
}

fn handle_stream(mpsc_sender: Sender<MpscMessage>, stream: Arc<TcpStream>) -> Result<()> {
    let addr = stream.peer_addr()?;
    let send_error = mpsc_sender.clone();

    stream
        .as_ref()
        .set_read_timeout(Some(Duration::from_secs(5)))?;

    if let Err(err) = read_stream(mpsc_sender, stream, addr) {
        send_error.send(MpscMessage::Error(addr, format!("{err}")))?;
    }

    Ok(())
}

fn read_stream(
    mpsc_sender: Sender<MpscMessage>,
    stream: Arc<TcpStream>,
    addr: SocketAddr,
) -> Result<()> {
    let mut buf = [0u8; 1024];

    mpsc_sender.send(MpscMessage::Connect(addr, stream.clone()))?;

    loop {
        let bytes = stream.as_ref().read(&mut buf)?;
        let msg = TcpMessage::from_bytes(&buf[0..bytes])?;
        let is_leaving = msg.is_leaving();

        mpsc_sender.send(MpscMessage::Message(addr, msg))?;

        if is_leaving {
            break;
        }
    }

    mpsc_sender.send(MpscMessage::Disconnect(addr))?;

    Ok(())
}

pub fn serve(
    port: Option<u16>,
    shutdown_after_last: bool,
    silent: bool,
    print_port: bool,
) -> Result<()> {
    let port = port.unwrap_or(3000);

    let address = format!("127.0.0.1:{}", port);
    let tcp = TcpListener::bind(&address)?;

    if print_port {
        println!("{}", port);
    } else if !silent {
        println!("Server listening on {}", address);
    }

    let (mpsc_sender, mpsc_receiver) = channel::<MpscMessage>();
    let exit_tcp_listener = Arc::new(AtomicBool::new(false));

    let exit_spawn_thread = exit_tcp_listener.clone();
    let spawn_thread = thread::spawn(move || -> Result<()> {
        let mut threads: Vec<JoinHandle<Result<()>>> = Vec::new();
        for stream in tcp.incoming() {
            if exit_spawn_thread.load(Ordering::Relaxed) {
                for thread in threads {
                    thread.join().map_err(|err| anyhow!("{:?}", err))??;
                }

                return Ok(());
            } else {
                let stream = Arc::new(stream?);
                let tx = mpsc_sender.clone();

                threads.push(thread::spawn(move || handle_stream(tx, stream)));
            }
        }

        Ok(())
    });

    host_connections(mpsc_receiver, shutdown_after_last, silent)?;
    exit_tcp_listener.store(true, Ordering::Relaxed);
    // Force tcp incoming to wake up and
    TcpStream::connect(&address)?;
    spawn_thread.join().map_err(|err| anyhow!("{:?}", err))??;

    Ok(())
}
