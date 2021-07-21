use std::net::TcpStream;
use std::time::SystemTime;

pub enum ClientEvent {
    Connection {
        stream: TcpStream,
    },
    ReadBlockchainRequest {},
    WriteBlockchainRequest {
        // transaction: Transaction,
    },
    // LockRequest {
    //     read_only: bool,
    //     request_id: u32,
    // },
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
    ReadBlockchainResponse {
        approved: bool, //cambiar por blockchain
    },
    // ReleaseLock {},
}
