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
        message: Message,
    },
    LeaderEvent {
        message: LeaderMessage,
        peer_id: PeerIdType,
    },
}
impl ClientEvent {
    pub fn serialize(&self) -> String {
        match self {
            ClientEvent::UserInput { message } => match message {
                Message::Common(message) => message.serialize(),
                Message::Leader(message) => message.serialize(),
            },
            ClientEvent::LeaderEvent {
                message,
                peer_id: _,
            } => message.serialize(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Common(ClientMessage),
    Leader(LeaderMessage),
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest {},
    ReadBlockchainResponse { blockchain: Blockchain },
    WriteBlockchainRequest { transaction: Transaction },
    WriteBlockchainResponse {},
    LockRequest { read_only: bool },
    LockResponse { acquired: bool },
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
            ClientMessage::ErrorResponse {
                msg: ErrorMessage::NotLeaderError,
            } => "error not_leader".to_owned(),
            ClientMessage::ErrorResponse {
                msg: ErrorMessage::LockNotAcquiredError,
            } => "error not_locked".to_owned(),
        }
    }

    pub fn deserialize(line: &String) -> Option<ClientMessage> {
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
    type Item = Message;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut line = String::new();
        self.reader.read_line(&mut line).ok()?;
        if let Some(message) = ClientMessage::deserialize(&line) {
            Some(Message::Common(message))
        } else {
            let message = LeaderMessage::deserialize(&line)?;
            Some(Message::Leader(message))
        }
    }
}

#[derive(Clone, Debug)]
pub enum LeaderMessage {
    LeaderElectionRequest { timestamp: SystemTime },
    CurrentLeaderLocal { response_sender: Sender<PeerIdType> },
    OkMessage,
    VictoryMessage {},
}
impl LeaderMessage {
    pub fn serialize(&self) -> String {
        match self {
            LeaderMessage::LeaderElectionRequest { timestamp } => {
                let time_epoch = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap();
                format!("le {}", time_epoch.as_secs())
            }
            LeaderMessage::CurrentLeaderLocal { .. } => {
                unreachable!()
            }
            LeaderMessage::OkMessage {} => "ok".to_owned(),
            LeaderMessage::VictoryMessage {} => {
                // TODO: usar timestamp
                "coordinator".to_owned()
            }
        }
    }

    pub fn deserialize(line: &String) -> Option<LeaderMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("le") => LeaderMessage::parse_leader_req(&mut tokens),
            Some("coordinator") => Some(LeaderMessage::VictoryMessage {}),
            _ => None,
        }
    }

    fn parse_leader_req(tokens: &mut dyn Iterator<Item = &str>) -> Option<LeaderMessage> {
        let _timestamp_str = tokens.next(); //TODO: pasar a timestamp si lo vamos a usar
        Some(LeaderMessage::LeaderElectionRequest {
            timestamp: SystemTime::now(),
        })
    }
}
