use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{ClientEvent, ClientMessage, Message, LockMessage};
use std::ops::Deref;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct LockProcessor {
    peer_handler_sender: Sender<ClientEvent>,
    lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
}

impl LockProcessor {
    pub fn new(
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        LockProcessor {
            peer_handler_sender,
            lock_notify,
        }
    }

    pub fn handle(&self, message: LockMessage, peer_id: PeerIdType) {
        match message {
            LockMessage::Acquire => self.acquire(peer_id),
            LockMessage::Release => self.release(peer_id),
        }
    }

    pub fn acquire(&self, peer_id: PeerIdType) {
        debug!("[{}] Acquiring lock", peer_id);
        let (mutex, cv) = self.lock_notify.deref();
        let acquired;
        if let Ok(leader_lock) = mutex.lock() {
            let timeout = leader_lock.get_duration();
            match cv.wait_timeout_while(leader_lock, timeout, |lock| !lock.is_used()) {
                Ok(mut guard) => {
                    acquired = guard.0.acquire(peer_id) == LockResult::Acquired;
                }
                _ => acquired = false,
            }
        } else {
            acquired = false
        }
        let message = Message::Common(ClientMessage::LockResponse(acquired));
        self.peer_handler_sender
            .send(ClientEvent::PeerMessage { message, peer_id })
            .ok();
    }

    pub fn release(&self, peer_id: PeerIdType) {
        info!("Release from {}", peer_id);
        let (guard, cv) = self.lock_notify.deref();
        if let Ok(mut lock) = guard.lock() {
            lock.release(peer_id);
            cv.notify_all();
        }
    }

    pub fn is_owned_by(&self, peer_id: PeerIdType) -> bool {
        if let Ok(guard) = self.lock_notify.0.lock() {
            return guard.is_owned_by(peer_id);
        }
        false
    }
}
