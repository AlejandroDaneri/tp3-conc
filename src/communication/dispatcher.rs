use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{
    ClientEvent, ClientMessage, LeaderMessage, LockMessage, Message,
};
use crate::handler::lock_handler::LockProcessor;
use std::io;
use std::sync::mpsc::{channel, Sender};

#[derive(Clone)]
pub struct Dispatcher {
    id: PeerIdType,
    pub peer_sender: Sender<ClientEvent>,
    pub message_sender: Sender<(ClientMessage, PeerIdType)>,
    pub leader_sender: Sender<(LeaderMessage, PeerIdType)>,
    pub output_sender: Sender<ClientMessage>,
    lock_handler: LockProcessor,
}

impl Dispatcher {
    pub fn new(
        id: PeerIdType,
        peer_sender: Sender<ClientEvent>,
        message_sender: Sender<(ClientMessage, PeerIdType)>,
        leader_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
        lock_handler: LockProcessor,
    ) -> Self {
        Self {
            id,
            peer_sender,
            message_sender,
            leader_sender,
            output_sender,
            lock_handler,
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
                        self.lock_handler.acquire(peer_id)
                    }
                    LockMessage::Release => self.lock_handler.release(peer_id),
                },
            },
            ClientEvent::UserInput { message } => match &message {
                Message::Common(inner) => {
                    let leader_id = Dispatcher::retrieve_leader(&self.leader_sender);
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
                        self.lock_handler.handle(message.clone(), self.id);
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

    pub fn is_lock_owned_by(&self, peer_id: PeerIdType) -> bool {
        self.lock_handler.is_owned_by(peer_id)
    }
}
