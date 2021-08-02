use crate::blockchain::lock::LockResult::{Acquired, Locked, ReleaseFailed, Released};
use crate::blockchain::peer::PeerIdType;

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
}

#[derive(Debug)]
pub struct CentralizedLock {
    peer_id: Option<PeerIdType>,
}

impl Lock for CentralizedLock {
    //envio pedido de acquire al coordinador
    fn acquire(&mut self, peer_id: PeerIdType) -> LockResult {
        if self.peer_id.is_none() {
            self.peer_id = Some(peer_id);
            Acquired
        } else {
            Locked
        }
    }

    //envio pedido de release al coordinador
    fn release(&mut self, peer_id: PeerIdType) -> LockResult {
        if self.is_owned_by(peer_id) {
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
        self.peer_id == Some(peer_id)
    }
}

impl CentralizedLock {
    pub fn new() -> CentralizedLock {
        CentralizedLock { peer_id: None }
    }
}

impl Default for CentralizedLock {
    fn default() -> Self {
        Self::new()
    }
}
