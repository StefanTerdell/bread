use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, RecvError, SendError, Sender};
use std::sync::Arc;
use std::thread;

use crate::TcpMessage;

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
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> ServerError {
        return ServerError::Io(err);
    }
}

impl From<SendError<MpscMessage>> for ServerError {
    fn from(err: SendError<MpscMessage>) -> ServerError {
        return ServerError::MpscS(err);
    }
}

impl From<RecvError> for ServerError {
    fn from(err: RecvError) -> ServerError {
        return ServerError::MpscR(err);
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
            MpscMessage::Message(address, msg) => {
                let msg = msg.to_bytes();

                for (other_address, other_stream) in streams.iter() {
                    if *other_address == address {
                        continue;
                    }

                    other_stream.as_ref().write(&msg)?;
                }
            }
            MpscMessage::Disconnect(address) => {
                streams.remove(&address);

                if !silent {
                    println!("Client disconnected. Current count: {}", streams.len());
                }

                if streams.len() == 0 && shutdown_after_last {
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
        let msg = TcpMessage::from_bytes(&buf[0..bytes]);
        let is_leaving = &msg == &TcpMessage::Leaving;

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
) -> Result<(), ServerError> {
    let port = port.unwrap_or(3000);

    let tcp = TcpListener::bind(format!("127.0.0.1:{}", port))?;

    println!("{}", port);

    let (mpsc_sender, mpsc_receiver) = channel::<MpscMessage>();

    thread::spawn(move || {
        for stream in tcp.incoming() {
            let stream = Arc::new(stream.unwrap());
            let tx = mpsc_sender.clone();
            thread::spawn(move || stream_handler(tx, stream));
        }
    });

    stream_host(mpsc_receiver, shutdown_after_last, silent)?;

    Ok(())
}
