use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

const ADDRESS: &str = "127.0.0.1:42069";

enum Event {
    Connect(SocketAddr, Arc<TcpStream>),
    Message(SocketAddr, Message),
    Disconnect(SocketAddr),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum Message {
    Chat(String),
    Leaving,
    Nothing,
}

impl Message {
    fn from_bytes(buf: &[u8]) -> Message {
        let mut de = Deserializer::new(Cursor::new(buf));
        let message: Message = Deserialize::deserialize(&mut de).unwrap();

        return message;
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut se = Serializer::new(&mut buf);
        self.serialize(&mut se).unwrap();

        return buf;
    }
}

fn trim_end(str: &String, n: usize) -> String {
    return str[0..str.len() - n].to_string();
}

fn host_writer(rx: Receiver<Event>) {
    let mut streams = HashMap::new();

    loop {
        match rx.recv().unwrap() {
            Event::Connect(address, stream) => {
                streams.insert(address, stream);
            }
            Event::Message(address, msg) => {
                let msg = msg.to_bytes();

                for (other_address, other_stream) in streams.iter() {
                    if *other_address == address {
                        continue;
                    }

                    other_stream.as_ref().write(&msg).unwrap();
                }
            }
            Event::Disconnect(address) => {
                streams.remove(&address);
            }
        };
    }
}

fn host_reader(tx: Sender<Event>, stream: Arc<TcpStream>) {
    let mut buf = [0u8; 1024];
    let addr = stream.peer_addr().unwrap();

    tx.send(Event::Connect(addr, stream.clone())).unwrap();

    loop {
        let bytes = stream.as_ref().read(&mut buf).unwrap();
        let msg = Message::from_bytes(&buf[0..bytes]);
        let is_leaving = &msg == &Message::Leaving;

        tx.send(Event::Message(addr, msg)).unwrap();

        if is_leaving {
            break;
        }
    }

    tx.send(Event::Disconnect(addr)).unwrap();
}

fn host() {
    let tcp = TcpListener::bind(ADDRESS).unwrap();
    let (tx, rx) = channel::<Event>();

    thread::spawn(move || host_writer(rx));

    thread::spawn(move || {
        for stream in tcp.incoming() {
            let tx = tx.clone();
            let stream = Arc::new(stream.unwrap());
            thread::spawn(move || host_reader(tx, stream));
        }
    });

    client(TcpStream::connect(ADDRESS).unwrap());
}

fn client(stream: TcpStream) {
    let stream = Arc::new(stream);

    let read_stream = stream.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match read_stream.as_ref().read(&mut buf) {
                Ok(bytes) => {
                    let msg = Message::from_bytes(&buf[0..bytes]);

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
        io::stdin().read_line(&mut buf).unwrap();
        buf.truncate(buf.len() - 2);

        let msg = if buf.len() == 0 {
            Message::Nothing
        } else if buf == ".exit" {
            Message::Leaving
        } else {
            Message::Chat(buf.clone())
        };

        if let Err(err) = stream.as_ref().write(&msg.to_bytes()) {}

        if msg == Message::Leaving {
            break;
        }
    }
}

fn main() {
    let stream = TcpStream::connect(ADDRESS);

    if let Ok(stream) = stream {
        client(stream);
    } else {
        host();
    }
}
