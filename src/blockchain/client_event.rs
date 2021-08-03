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

#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest {},
    ReadBlockchainResponse { blockchain: Blockchain },
    WriteBlockchainRequest { transaction: Transaction },
    WriteBlockchainResponse {},
    LockRequest { read_only: bool },
    LockResponse { acquired: bool },
    StillAlive {},
    ErrorResponse { msg: ErrorMessage },
}

#[derive(Clone, Debug)]
pub enum ErrorMessage {
    NotLeaderError,
    LockNotAcquiredError,
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
            ClientMessage::WriteBlockchainResponse {} => "wb_response".to_owned(),
            ClientMessage::LockRequest { read_only } => {
                if *read_only {
                    "lock read".to_owned()
                } else {
                    "lock write".to_owned()
                }
            }
            ClientMessage::LockResponse { acquired } => {
                if *acquired {
                    "lock acquired".to_owned()
                } else {
                    "lock failed".to_owned()
                }
            }
            ClientMessage::StillAlive {} => "alive".to_owned(),
            ClientMessage::ErrorResponse {
                msg: ErrorMessage::NotLeaderError,
            } => "error not_leader".to_owned(),
            ClientMessage::ErrorResponse {
                msg: ErrorMessage::LockNotAcquiredError,
            } => "error not_locked".to_owned(),
        }
    }

    pub fn deserialize(line: String) -> Option<ClientMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientMessage::ReadBlockchainRequest {}),
            Some("wb") => ClientMessage::parse_write_blockchain(&mut tokens),
            Some("lock") => ClientMessage::parse_lock(&mut tokens),
            Some("blockchain") => ClientMessage::parse_blockchain(&mut tokens),
            Some("error") => ClientMessage::parse_error(&mut tokens),
            _ => None,
        }
    }

    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientMessage::WriteBlockchainRequest { transaction })
    }

    fn parse_lock(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let read_only_str = tokens.next()?;
        match read_only_str {
            "read" => Some(ClientMessage::LockRequest { read_only: true }),
            "write" => Some(ClientMessage::LockRequest { read_only: false }),
            "acquired" => Some(ClientMessage::LockResponse { acquired: true }),
            "failed" => Some(ClientMessage::LockResponse { acquired: false }),
            _ => None,
        }
    }

    fn parse_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        Some(ClientMessage::ReadBlockchainResponse {
            blockchain: Blockchain::parse(tokens)?,
        })
    }

    fn parse_error(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let error_str = tokens.next()?;
        match error_str {
            "not_leader" => Some(ClientMessage::ErrorResponse {
                msg: ErrorMessage::NotLeaderError,
            }),
            "not_locked" => Some(ClientMessage::ErrorResponse {
                msg: ErrorMessage::LockNotAcquiredError,
            }),
            _ => None,
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
    TodoMessage {
        msg: String,
    },
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
            LeaderMessage::TodoMessage { msg: _ } => todo!(),
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
        let _timestamp_str = tokens.next()?; //pasar a timestamp
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
