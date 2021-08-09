use crate::blockchain::lock::LockResult::{Acquired, Locked, ReleaseFailed, Released};
use crate::blockchain::peer::PeerIdType;
use std::time::SystemTime;

const LOCK_EXPIRATION_TIME: u64 = 5;

#[derive(PartialEq)]
pub enum LockResult {
    Acquired,
    Locked,
    Released,
    ReleaseFailed,
}

pub trait Lock {
    fn acquire(&mut self, peer_id: PeerIdType) -> LockResult;

    fn release(&mut self, peer_id: PeerIdType) -> LockResult;

    fn reset(&mut self);

    fn is_owned_by(&self, peer_id: PeerIdType) -> bool;

    fn lock_expired(&self) -> bool;

    fn get_duration(&self) -> u64;
}

#[derive(Debug)]
pub struct CentralizedLock {
    peer_id: Option<PeerIdType>,
    lock_time: SystemTime,
    expiration_time: u64
}

impl Lock for CentralizedLock {
    //envio pedido de acquire al coordinador
    fn acquire(&mut self, peer_id: PeerIdType) -> LockResult {
        if self.peer_id.is_none() || self.lock_expired() {
            self.peer_id = Some(peer_id);
            self.lock_time = SystemTime::now();
            Acquired
        } else {
            Locked
        }
    }

    //envio pedido de release al coordinador
    fn release(&mut self, peer_id: PeerIdType) -> LockResult {
        if self.is_owned_by(peer_id) && !self.lock_expired() {
            self.peer_id = None;
            Released
        } else {
            ReleaseFailed
        }
    }

    fn reset(&mut self) {
        self.peer_id = None;
    }

    fn is_owned_by(&self, peer_id: PeerIdType) -> bool {
        self.peer_id == Some(peer_id) && !self.lock_expired()
    }

    fn lock_expired(&self) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.lock_time) {
            elapsed.as_secs() > self.expiration_time
        } else {
            true
        }
    }

    fn get_duration(&self) -> u64 {
        self.expiration_time
    }
}

impl CentralizedLock {
    pub fn new() -> CentralizedLock {
        CentralizedLock {
            peer_id: None,
            lock_time: SystemTime::UNIX_EPOCH,
            expiration_time: LOCK_EXPIRATION_TIME
        }
    }
}

impl Default for CentralizedLock {
    fn default() -> Self {
        Self::new()
    }
}
