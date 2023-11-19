use crate::tcp_message::TcpMessageError;
use crate::TcpMessage;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvError, SendError, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

#[derive(Debug)]
pub enum MpscMessage {
    Connect(SocketAddr, Arc<TcpStream>),
    Message(SocketAddr, TcpMessage),
    Disconnect(SocketAddr),
}

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    MpscS(SendError<MpscMessage>),
    MpscR(RecvError),
    Message(TcpMessageError),
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> ServerError {
        ServerError::Io(err)
    }
}

impl From<SendError<MpscMessage>> for ServerError {
    fn from(err: SendError<MpscMessage>) -> ServerError {
        ServerError::MpscS(err)
    }
}

impl From<RecvError> for ServerError {
    fn from(err: RecvError) -> ServerError {
        ServerError::MpscR(err)
    }
}

impl From<TcpMessageError> for ServerError {
    fn from(err: TcpMessageError) -> ServerError {
        ServerError::Message(err)
    }
}

fn stream_host(
    mpsc_receiver: Receiver<MpscMessage>,
    shutdown_after_last: bool,
    silent: bool,
) -> Result<(), ServerError> {
    let mut streams = HashMap::new();

    loop {
        match mpsc_receiver.recv()? {
            MpscMessage::Connect(address, stream) => {
                streams.insert(address, stream);

                if !silent {
                    println!("Client connected. Current count: {}", streams.len());
                }
            }
            MpscMessage::Message(_, msg) => {
                let msg = msg.to_bytes()?;

                for stream in streams.values() {
                    stream.as_ref().write_all(&msg)?;
                }
            }
            MpscMessage::Disconnect(address) => {
                streams.remove(&address);

                if !silent {
                    println!("Client disconnected. Current count: {}", streams.len());
                }

                if streams.is_empty() && shutdown_after_last {
                    if !silent {
                        println!("Last connection left! Shutting down 🦄");
                    }
                    break;
                }
            }
        };
    }

    Ok(())
}

fn stream_handler(
    mpsc_sender: Sender<MpscMessage>,
    stream: Arc<TcpStream>,
) -> Result<(), ServerError> {
    let mut buf = [0u8; 1024];
    let addr = stream.peer_addr()?;

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
) -> Result<(), ServerError> {
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
    let spawn_thread = thread::spawn(move || -> Result<(), ServerError> {
        let mut threads: Vec<JoinHandle<Result<(), ServerError>>> = Vec::new();
        for stream in tcp.incoming() {
            if exit_spawn_thread.load(Ordering::Relaxed) {
                for thread in threads {
                    thread.join().expect("🦄")?;
                }

                return Ok(());
            } else {
                let stream = Arc::new(stream?);
                let tx = mpsc_sender.clone();

                threads.push(thread::spawn(move || stream_handler(tx, stream)));
            }
        }

        Ok(())
    });

    stream_host(mpsc_receiver, shutdown_after_last, silent)?;
    exit_tcp_listener.store(true, Ordering::Relaxed);
    // Force tcp incoming to wake up and
    TcpStream::connect(&address)?;
    spawn_thread.join().expect("🦄")?;

    Ok(())
}
