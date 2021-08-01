use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::time::SystemTime;

use crate::blockchain::blockchain::{Blockchain, Transaction};
use crate::blockchain::peer::PeerIdType;

use super::blockchain::Block;

#[derive(Debug)]
pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
    PeerMessage {
        message: ClientMessage,
        peer_id: PeerIdType,
    },
    PeerDisconnected {
        peer_id: PeerIdType,
    },
    UserInput {
        message: ClientMessage,
    },
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest {},
    ReadBlockchainResponse {
        blockchain: Blockchain,
    },
    WriteBlockchainRequest {
        transaction: Transaction,
    },
    LeaderElectionRequest {
        request_id: u32,
        timestamp: SystemTime,
    },
    LockRequest {
        read_only: bool,
        request_id: u32,
    },
    OkMessage,
    CoordinatorMessage {
        connection_id: u32,
    },
    StillAlive {},
    TodoMessage {
        msg: String,
    },
}

impl ClientMessage {
    pub fn serialize(&self) -> String {
        match self {
            ClientMessage::ReadBlockchainRequest {} => "rb".to_owned(),
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                format!("blockchain {}", blockchain.serialize())
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                format!("wb {}", transaction.serialize())
            }
            ClientMessage::LeaderElectionRequest {
                request_id,
                timestamp,
            } => {
                let time_epoch = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap();
                format!("le {} {}", request_id, time_epoch.as_secs())
            }
            ClientMessage::LockRequest {
                read_only,
                request_id,
            } => {
                if *read_only {
                    format!("lock read {}", request_id)
                } else {
                    format!("lock write {}", request_id)
                }
            }
            ClientMessage::OkMessage {} => format!("ok"),
            ClientMessage::CoordinatorMessage { connection_id } => {
                format!("coordinator {}", connection_id)
            }
            ClientMessage::StillAlive {} => format!("alive"),
            ClientMessage::TodoMessage { msg } => format!("TODO! {}", msg),
        }
    }

    pub fn deserialize(line: String) -> Option<ClientMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientMessage::ReadBlockchainRequest {}),
            Some("wb") => ClientMessage::parse_write_blockchain(&mut tokens),
            Some("le") => ClientMessage::parse_leader_req(&mut tokens),
            Some("lock") => ClientMessage::parse_lock_req(&mut tokens),
            Some("coordinator") => Some(ClientMessage::parse_coord(&mut tokens)),
            Some("blockchain") => Some(ClientMessage::parse_blockchain(&mut tokens)),
            _ => None,
        }
    }

    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientMessage::WriteBlockchainRequest { transaction })
    }

    fn parse_leader_req(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let request_id_str = tokens.next()?;
        let _timestamp_str = tokens.next()?; //pasar a timestamp
        Some(ClientMessage::LeaderElectionRequest {
            request_id: request_id_str.parse::<u32>().ok()?,
            timestamp: SystemTime::now(),
        })
    }

    fn parse_lock_req(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let read_only_str = tokens.next()?;
        let request_id_str = tokens.next()?;
        Some(ClientMessage::LockRequest {
            read_only: read_only_str.parse::<bool>().ok()?,
            request_id: request_id_str.parse::<u32>().ok()?,
        })
    }

    fn parse_coord(tokens: &mut dyn Iterator<Item = &str>) -> ClientMessage {
        let new_leader_id = tokens.next().unwrap();
        ClientMessage::CoordinatorMessage {
            connection_id: new_leader_id.parse::<u32>().unwrap(),
        }
    }

    fn parse_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> ClientMessage {
        ClientMessage::ReadBlockchainResponse {
            blockchain: Blockchain::parse(tokens),
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
}

impl<R: Read> Iterator for ClientEventReader<R> {
    type Item = ClientMessage;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut line = String::new();
        self.reader.read_line(&mut line).ok()?;
        ClientMessage::deserialize(line)
    }
}
