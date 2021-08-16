use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{ClientEvent, ClientMessage, Message};
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub struct LockHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct LockProcessor {
    message_receiver: Receiver<PeerIdType>, // Sólo llegan mensajes de acquire
    peer_handler_sender: Sender<ClientEvent>,
    lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
}

impl LockHandler {
    pub fn new(
        lock_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LockHandler::run(lock_receiver, peer_handler_sender, lock_notify);
        }));
        LockHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) {
        let mut processor = LockProcessor::new(message_receiver, peer_handler_sender, lock_notify);
        processor.run();
    }
}

impl LockProcessor {
    pub fn new(
        message_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        LockProcessor {
            message_receiver,
            peer_handler_sender,
            lock_notify,
        }
    }

    pub fn run(&mut self) {
        while let Ok(peer_id) = self.message_receiver.recv() {
            let (mutex, cv) = self.lock_notify.deref();
            let acquired;
            if let Ok(leader_lock) = mutex.lock() {
                debug!("Taking CV...");
                let timeout = Duration::from_secs(leader_lock.get_duration());
                match cv.wait_timeout_while(leader_lock, timeout, |lock| lock.is_owned_by(peer_id))
                {
                    Ok(mut guard) => {
                        acquired = guard.0.acquire(peer_id) == LockResult::Acquired;
                        debug!("wait_timeout: {}", acquired);
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
    }
}
impl Drop for LockHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
