use anyhow::Result;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, net::SocketAddr};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TcpMessage {
    Chat(SocketAddr, String),
    Leaving(SocketAddr),
    Nothing(SocketAddr),
}

impl TcpMessage {
    pub fn from_bytes(buf: &[u8]) -> Result<TcpMessage> {
        let mut de = Deserializer::new(Cursor::new(buf));
        let message: TcpMessage = Deserialize::deserialize(&mut de)?;

        Ok(message)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut se = Serializer::new(&mut buf);
        self.serialize(&mut se)?;

        Ok(buf)
    }

    pub fn is_leaving(&self) -> bool {
        matches!(self, TcpMessage::Leaving(_))
    }

    pub fn get_address(&self) -> SocketAddr {
        match self {
            TcpMessage::Chat(addr, _) => *addr,
            TcpMessage::Leaving(addr) => *addr,
            TcpMessage::Nothing(addr) => *addr,
        }
    }
}
