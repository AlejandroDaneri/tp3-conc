use std::net::TcpStream;
use std::time::SystemTime;

use super::blockchain::Transaction;

pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
    ReadBlockchainRequest {
        request_id: u16,
    },
    WriteBlockchainRequest {
        request_id: u16,
        transaction: Transaction,
    },
    LeaderElectionRequest {
        request_id: u32,
        timestamp: SystemTime,
    },
    ConnectionError {
        connection_id: usize,
    },
    OkMessage {},
    CoordinatorMessage {
        new_leader_id: u32,
    },
}
