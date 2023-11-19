use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TcpMessage {
    Chat(String),
    Leaving,
    Nothing,
}

impl TcpMessage {
    pub fn from_bytes(buf: &[u8]) -> TcpMessage {
        let mut de = Deserializer::new(Cursor::new(buf));
        let message: TcpMessage = Deserialize::deserialize(&mut de).unwrap();

        return message;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut se = Serializer::new(&mut buf);
        self.serialize(&mut se).unwrap();

        return buf;
    }
}
