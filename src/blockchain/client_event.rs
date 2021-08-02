use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::time::SystemTime;

use crate::blockchain::blockchain::{Blockchain, Transaction};
use crate::blockchain::peer::PeerIdType;
use std::sync::mpsc::Sender;

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
    LeaderEvent {
        message: LeaderMessage,
    },
}
impl ClientEvent {
    pub fn serialize(&self) -> String {
        match self {
            ClientEvent::UserInput { message } => message.serialize(),
            ClientEvent::LeaderEvent { message } => message.serialize(),
            _ => unreachable!(),
        }
    }
}
#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest {},
    ReadBlockchainResponse { blockchain: Blockchain },
    WriteBlockchainRequest { transaction: Transaction },
    LockRequest { read_only: bool },
    StillAlive {},
    TodoMessage { msg: String },
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
            ClientMessage::LockRequest { read_only } => {
                if *read_only {
                    "lock true".to_owned()
                } else {
                    "lock false".to_owned()
                }
            }
            ClientMessage::StillAlive {} => "alive".to_owned(),
            ClientMessage::TodoMessage { msg } => format!("TODO! {}", msg),
        }
    }

    pub fn deserialize(line: String) -> Option<ClientMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientMessage::ReadBlockchainRequest {}),
            Some("wb") => ClientMessage::parse_write_blockchain(&mut tokens),
            Some("lock") => ClientMessage::parse_lock_req(&mut tokens),
            Some("blockchain") => Some(ClientMessage::parse_blockchain(&mut tokens)),
            _ => None,
        }
    }

    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientMessage::WriteBlockchainRequest { transaction })
    }

    fn parse_lock_req(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let read_only_str = tokens.next()?;
        Some(ClientMessage::LockRequest {
            read_only: read_only_str.parse::<bool>().ok()?,
        })
    }

    fn parse_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> ClientMessage {
        ClientMessage::ReadBlockchainResponse {
            blockchain: Blockchain::parse(tokens),
        }
    }
}

pub struct ClientEventReader<R> {
    reader: BufReader<R>,
}

impl<R: Read> ClientEventReader<R> {
    pub fn new(source: R) -> Self {
        let reader = BufReader::new(source);
        Self { reader }
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

#[derive(Clone, Debug)]
pub enum LeaderMessage {
    LeaderElectionRequest {
        request_id: PeerIdType,
        timestamp: SystemTime,
    },
    CurrentLeaderLocal {
        response_sender: Sender<PeerIdType>,
    },
    OkMessage,
    CoordinatorMessage {
        connection_id: u32,
    },
    StillAlive {},
}
impl LeaderMessage {
    pub fn serialize(&self) -> String {
        match self {
            LeaderMessage::LeaderElectionRequest {
                request_id,
                timestamp,
            } => {
                let time_epoch = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap();
                format!("le {} {}", request_id, time_epoch.as_secs())
            }
            LeaderMessage::CurrentLeaderLocal { .. } => {
                unreachable!()
            }
            LeaderMessage::OkMessage {} => "ok".to_owned(),
            LeaderMessage::CoordinatorMessage { connection_id } => {
                format!("coordinator {}", connection_id)
            }
            LeaderMessage::StillAlive {} => "alive".to_owned(),
        }
    }

    pub fn deserialize(line: String) -> Option<LeaderMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("le") => LeaderMessage::parse_leader_req(&mut tokens),
            Some("coordinator") => Some(LeaderMessage::parse_coord(&mut tokens)),
            _ => None,
        }
    }

    fn parse_leader_req(tokens: &mut dyn Iterator<Item = &str>) -> Option<LeaderMessage> {
        let request_id_str = tokens.next()?;
        let _timestamp_str = tokens.next()?; //TODO: pasar a timestamp si lo vamos a usar
        Some(LeaderMessage::LeaderElectionRequest {
            request_id: request_id_str.parse::<u32>().ok()?,
            timestamp: SystemTime::now(),
        })
    }

    fn parse_coord(tokens: &mut dyn Iterator<Item = &str>) -> LeaderMessage {
        let new_leader_id = tokens.next().unwrap();
        LeaderMessage::CoordinatorMessage {
            connection_id: new_leader_id.parse::<u32>().unwrap(),
        }
    }
}
