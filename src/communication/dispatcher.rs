use crate::blockchain::lock::{CentralizedLock, Lock};
use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{
    ClientEvent, ClientMessage, LeaderMessage, LockMessage, Message,
};
use std::io;
use std::ops::Deref;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct Dispatcher {
    id: PeerIdType,
    pub peer_sender: Sender<ClientEvent>,
    pub message_sender: Sender<(ClientMessage, PeerIdType)>,
    pub leader_sender: Sender<(LeaderMessage, PeerIdType)>,
    pub lock_sender: Sender<PeerIdType>,
    pub output_sender: Sender<ClientMessage>,
    pub lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
}

impl Dispatcher {
    pub fn new(
        id: PeerIdType,
        peer_sender: Sender<ClientEvent>,
        message_sender: Sender<(ClientMessage, PeerIdType)>,
        leader_sender: Sender<(LeaderMessage, PeerIdType)>,
        lock_sender: Sender<PeerIdType>,
        output_sender: Sender<ClientMessage>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        Self {
            id,
            peer_sender,
            message_sender,
            leader_sender,
            lock_sender,
            output_sender,
            lock_notify,
        }
    }

    pub fn dispatch(&self, event: ClientEvent) -> io::Result<()> {
        match event {
            ClientEvent::Connection { .. } | ClientEvent::PeerDisconnected { .. } => {
                self.peer_sender
                    .send(event)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "peer sender error"))?;
            }
            ClientEvent::PeerMessage { message, peer_id } => match message {
                Message::Common(message) => {
                    self.message_sender.send((message, peer_id)).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "message sender error")
                    })?;
                }
                Message::Leader(message) => {
                    self.leader_sender
                        .send((message, peer_id))
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "leader sender error"))?;
                }
                Message::Lock(message) => match message {
                    LockMessage::Acquire => {
                        println!("acquire to {}", peer_id);
                        self.lock_sender.send(peer_id).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "lock sender error")
                        })?;
                    }
                    LockMessage::Release => {
                        let (guard, cv) = self.lock_notify.deref();
                        if let Ok(mut lock) = guard.lock() {
                            lock.release(peer_id);
                            cv.notify_all();
                        }
                    }
                },
            },
            ClientEvent::UserInput { message } => match &message {
                Message::Common(inner) => {
                    let leader_id = Dispatcher::retrieve_leader(&self.leader_sender);
                    debug!("Leader retrieved");
                    if leader_id != self.id {
                        let event = ClientEvent::PeerMessage {
                            message,
                            peer_id: leader_id,
                        };
                        self.peer_sender.send(event).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "message sender error")
                        })?;
                    } else {
                        self.message_sender
                            .send((inner.clone(), self.id))
                            .map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "message sender error")
                            })?;
                    }
                }
                Message::Leader(message) => {
                    self.leader_sender
                        .send((message.clone(), 0))
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "leader sender error"))?;
                }
                Message::Lock(message) => {
                    let leader_id = Dispatcher::retrieve_leader(&self.leader_sender);
                    if leader_id == self.id {
                        self.lock_sender.send(self.id).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "lock sender error")
                        })?;
                    } else {
                        let event = ClientEvent::PeerMessage {
                            message: Message::Lock(message.clone()),
                            peer_id: leader_id,
                        };
                        self.peer_sender.send(event).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "peer sender error")
                        })?;
                    }
                }
            },
        }
        Ok(())
    }

    fn retrieve_leader(leader_sender: &Sender<(LeaderMessage, PeerIdType)>) -> PeerIdType {
        let (response_sender, response_receiver) = channel();
        let message = LeaderMessage::CurrentLeaderLocal { response_sender };
        leader_sender.send((message, 0)).unwrap();
        response_receiver.recv().unwrap()
    }
}
