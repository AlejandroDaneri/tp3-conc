use std::net::TcpStream;
use std::time::SystemTime;

use super::blockchain::Transaction;

pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
    ReadBlockchainRequest {},
    WriteBlockchainRequest {
        transaction: Transaction,
    },
    LockRequest {
        read_only: bool,
        request_id: u32,
    },
    LeaderElectionRequest {
        request_id: u32,
        timestamp: SystemTime,
    },
    ConnectionError {
        connection_id: usize,
    },
}
