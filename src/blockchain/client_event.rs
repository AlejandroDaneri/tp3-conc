use std::io::{BufReader, Read, BufRead};
use std::net::TcpStream;
use std::time::SystemTime;

use super::blockchain::Transaction;
use crate::blockchain::client_event::ClientEvent::{ConnectionError, ReadBlockchainRequest};
use std::str::SplitWhitespace;

#[derive(Debug)]
pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
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
    }
}

pub struct ClientEventReader<R> {
    reader: BufReader<R>,
    client_id: u32
}

impl<R: Read> ClientEventReader<R> {
    pub fn new(source: R, client_id: u32) -> Self {
        let reader = BufReader::new(source);
        Self { reader, client_id }
    }

    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientEvent> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientEvent::WriteBlockchainRequest {transaction})
    }
}

impl<R: Read> Iterator for ClientEventReader<R> {
    type Item = ClientEvent;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut line = String::new();
        self.reader.read_line(&mut line);
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientEvent::ReadBlockchainRequest{}),
            Some("wb") => ClientEventReader::<R>::parse_write_blockchain(&mut tokens),
            _ => None
        }
    }
}
