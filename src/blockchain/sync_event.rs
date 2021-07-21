use super::blockchain::Transaction;

pub enum SyncEvent {
    LockRequest {
        read_only: bool,
        request_id: u32,
    },
    ReadBlockchainResponse {
        approved: bool, //cambiar por blockchain
    },
    WriteBlockchainResponse {
        approved: bool, //cambiar por blockchain
    },
    ReleaseLock {
        transaction: Transaction,
    },
}
