use std::io;
use std::thread;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{Sender, Receiver};
use crate::blockchain::peer::PeerIdType;
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::communication::client_event::{ClientEvent, ClientMessage, Message};
use std::ops::Deref;
use std::time::Duration;

pub struct LockHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct LockProcessor {
    message_receiver: Receiver<PeerIdType>, // Sólo llegan mensajes de acquire
    peer_handler_sender: Sender<ClientEvent>,
    lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>
}

impl LockHandler {
    pub fn new(
        lock_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LockHandler::run(
                lock_receiver,
                peer_handler_sender,
                lock_notify
            )
                .unwrap();
        }));
        LockHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>
    ) -> io::Result<()> {
        let mut processor = LockProcessor::new(message_receiver, peer_handler_sender, lock_notify);
        processor.run();
        Ok(())
    }
}

impl LockProcessor {
    pub fn new(
        message_receiver: Receiver<PeerIdType>,
        peer_handler_sender: Sender<ClientEvent>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>
    ) -> Self {
        LockProcessor {
            message_receiver,
            peer_handler_sender,
            lock_notify
        }
    }

    pub fn run(&mut self) {
        while let Ok(peer_id) = self.message_receiver.recv() {
            let (mutex, cv) = self.lock_notify.deref();
            let acquired;
            if let Ok(leader_lock) = mutex.lock() {
                println!("Taking CV...");
                let timeout= Duration::from_secs(leader_lock.get_duration());
                match cv.wait_timeout_while(leader_lock, timeout,|lock| lock.is_owned_by(peer_id)) {
                    Ok(mut guard) => {
                        acquired = guard.0.acquire(peer_id) == LockResult::Acquired;
                    }
                    _ => {
                        acquired = false
                    }
                }
            } else {
                acquired = false
            }
            let message = Message::Common(ClientMessage::LockResponse(acquired));
            println!("Send lock ok...");
            self.peer_handler_sender.send(ClientEvent::PeerMessage { message, peer_id });
        }
    }
}