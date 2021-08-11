use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::time::SystemTime;

use crate::blockchain::blockchain::{Blockchain, Transaction};
use crate::blockchain::peer::PeerIdType;
use crate::communication::serialization::Serializable;

#[derive(Debug)]
pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
    PeerMessage {
        message: Message,
        peer_id: PeerIdType,
    },
    PeerDisconnected {
        peer_id: PeerIdType,
    },
    UserInput {
        message: Message,
    },
}
impl ClientEvent {
    pub fn serialize(&self) -> String {
        match self {
            ClientEvent::UserInput { message } => message.serialize(),
            ClientEvent::PeerMessage {
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
    Lock(LockMessage),
}

impl Serializable for Message {
    fn serialize(&self) -> String {
        match self {
            Message::Common(message) => message.serialize(),
            Message::Leader(message) => message.serialize(),
            Message::Lock(message) => message.serialize(),
        }
    }

    fn deserialize(line: &str) -> Option<Self> {
        if let Some(message) = ClientMessage::deserialize(&line) {
            Some(Message::Common(message))
        } else if let Some(message) = LeaderMessage::deserialize(&line) {
            Some(Message::Leader(message))
        } else {
            let message = LockMessage::deserialize(&line)?;
            Some(Message::Lock(message))
        }
    }
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    ReadBlockchainRequest,
    ReadBlockchainResponse { blockchain: Blockchain },
    WriteBlockchainRequest { transaction: Transaction },
    WriteBlockchainResponse {},
    LockResponse(bool),
    LeaderElectionFinished,
    ErrorResponse(ErrorMessage),
    BroadcastBlockchain { blockchain: Blockchain },
}

#[derive(Clone, Debug)]
pub enum ErrorMessage {
    NotLeaderError,
    LockNotAcquiredError,
}

impl Serializable for ClientMessage {
    fn serialize(&self) -> String {
        match self {
            ClientMessage::ReadBlockchainRequest {} => "rb\n".to_owned(),
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                format!("blockchain {}\n", blockchain.serialize())
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                format!("wb {}\n", transaction.serialize())
            }
            ClientMessage::WriteBlockchainResponse {} => "wb_response\n".to_owned(),
            ClientMessage::LockResponse(acquired) => {
                if *acquired {
                    "lock_ok\n".to_owned()
                } else {
                    "lock_failed\n".to_owned()
                }
            }
            ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError) => {
                "error not_leader\n".to_owned()
            }
            ClientMessage::ErrorResponse(ErrorMessage::LockNotAcquiredError) => {
                "error not_locked\n".to_owned()
            }
            ClientMessage::LeaderElectionFinished {} => {
                "info leader_election_finished\n".to_owned()
            }
            ClientMessage::BroadcastBlockchain { blockchain } => {
                format!("blockchain {}\n", blockchain.serialize())
            }
        }
    }

    fn deserialize(line: &str) -> Option<ClientMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(ClientMessage::ReadBlockchainRequest {}),
            Some("wb") => ClientMessage::parse_write_blockchain(&mut tokens),
            Some("wb_response") => Some(ClientMessage::WriteBlockchainResponse {}),
            Some("lock_failed") => Some(ClientMessage::LockResponse(false)),
            Some("lock_ok") => Some(ClientMessage::LockResponse(true)),
            Some("blockchain") => ClientMessage::parse_blockchain(&mut tokens),
            Some("error") => ClientMessage::parse_error(&mut tokens),
            _ => None,
        }
    }
}

impl ClientMessage {
    fn parse_write_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let transaction = Transaction::parse(tokens)?;
        Some(ClientMessage::WriteBlockchainRequest { transaction })
    }

    fn parse_blockchain(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        Some(ClientMessage::ReadBlockchainResponse {
            blockchain: Blockchain::parse(tokens)?,
        })
    }

    fn parse_error(tokens: &mut dyn Iterator<Item = &str>) -> Option<ClientMessage> {
        let error_str = tokens.next()?;
        match error_str {
            "not_leader" => Some(ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError)),
            "not_locked" => Some(ClientMessage::ErrorResponse(
                ErrorMessage::LockNotAcquiredError,
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LeaderMessage {
    LeaderElectionRequest { timestamp: SystemTime },
    CurrentLeaderLocal { response_sender: Sender<PeerIdType> },
    OkMessage,
    VictoryMessage,
    PeerDisconnected,
    SendLeaderId,
    BroadcastBlockchain { blockchain: Blockchain },
}

impl LeaderMessage {
    pub fn serialize(&self) -> String {
        match self {
            LeaderMessage::LeaderElectionRequest { timestamp } => {
                let time_epoch = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap();
                format!("le {}\n", time_epoch.as_secs())
            }
            LeaderMessage::CurrentLeaderLocal { .. } => {
                unreachable!()
            }
            LeaderMessage::OkMessage {} => "ok\n".to_owned(),
            LeaderMessage::VictoryMessage {} => "coordinator\n".to_owned(),
            LeaderMessage::PeerDisconnected => unreachable!(),
            LeaderMessage::SendLeaderId => unreachable!(),
            LeaderMessage::BroadcastBlockchain { blockchain: _ } => unreachable!(),
        }
    }

    pub fn deserialize(line: &str) -> Option<LeaderMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("le") => LeaderMessage::parse_leader_req(&mut tokens),
            Some("coordinator") => Some(LeaderMessage::VictoryMessage {}),
            Some("ok") => Some(LeaderMessage::OkMessage {}),
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

#[derive(Clone, Debug)]
pub enum LockMessage {
    Acquire,
    Release,
}

impl Serializable for LockMessage {
    fn serialize(&self) -> String {
        match self {
            LockMessage::Acquire => "lock_acquire\n".to_owned(),
            LockMessage::Release => "lock_release\n".to_owned(),
        }
    }

    fn deserialize(line: &str) -> Option<LockMessage> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("lock_acquire") => Some(LockMessage::Acquire),
            Some("lock_release") => Some(LockMessage::Release),
            _ => None,
        }
    }
}
