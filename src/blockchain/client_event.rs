use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::time::SystemTime;

use super::blockchain::Transaction;

#[derive(Debug)]
pub enum ClientEvent {
    Connection { stream: TcpStream },
    Message { message: ClientMessage },
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest {},
    WriteBlockchainRequest {
        transaction: Transaction,
    },
    LeaderElectionRequest {
        request_id: u32,
        timestamp: SystemTime,
    },
    ConnectionError {
        connection_id: u32,
    },
    OkMessage,
    CoordinatorMessage {
        connection_id: u32,
    },
    StillAlive {},
}

impl fmt::Display for ClientMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientMessage::ReadBlockchainRequest {} => write!(f, "rb"),
            ClientMessage::WriteBlockchainRequest { transaction } => write!(f, "wb {}", "test 10"),
            ClientMessage::LeaderElectionRequest {
                request_id,
                timestamp,
            } => {
                let time_epoch = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap();
                write!(f, "le {} {}", request_id, time_epoch.as_secs())
            }
            ClientMessage::ConnectionError { connection_id } => {
                write!(f, "error")
            }
            ClientMessage::OkMessage {} => write!(f, "ok"),
            ClientMessage::CoordinatorMessage { connection_id } => {
                write!(f, "coordinator {}", connection_id)
            }
            ClientMessage::StillAlive {} => write!(f, "alive"),
        }
    }
}

pub struct ClientEventReader<R> {
    reader: BufReader<R>,
    client_id: u32,
}

impl<R: Read> ClientEventReader<R> {
    pub fn new(source: R, client_id: u32) -> Self {
        let reader = BufReader::new(source);
        Self { reader, client_id }
    }

    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientMessage::WriteBlockchainRequest { transaction })
    }
}

impl<R: Read> Iterator for ClientEventReader<R> {
    type Item = ClientMessage;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut line = String::new();
        self.reader.read_line(&mut line).ok()?;
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientMessage::ReadBlockchainRequest {}),
            Some("wb") => ClientEventReader::<R>::parse_write_blockchain(&mut tokens),
            _ => None,
        }
    }
}
