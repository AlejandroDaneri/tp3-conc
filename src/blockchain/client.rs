use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{ClientEvent, ClientMessage, LockMessage, Message};
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputHandler;
use crate::handler::leader_handler::LeaderHandler;
use crate::handler::message_handler::MessageHandler;
use crate::handler::peer_handler::PeerHandler;

use crate::blockchain::lock::CentralizedLock;
use crate::communication::client_event::LeaderMessage;
use crate::handler::lock_handler::LockHandler;
use std::io::Read;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub struct Client {
    id: u32,
}

#[allow(clippy::mutex_atomic)]
impl Client {
    pub fn new(id: u32) -> Self {
        Client { id }
    }

    pub fn run<T: 'static + Read + Send>(
        &mut self,
        source: T,
        port_from: u16,
        port_to: u16,
    ) -> io::Result<()> {
        let (sender, receiver) = channel();

        let (leader_handler_sender, leader_handler_receiver) = channel();
        let (lock_handler_sender, lock_handler_receiver) = channel();
        let (peer_handler_sender, peer_handler_receiver) = channel();
        let (message_handler_sender, message_handler_receiver) = channel();
        let (output_sender, output_receiver) = channel();
        let peer_handler = PeerHandler::new(
            self.id,
            peer_handler_receiver,
            sender.clone(),
            leader_handler_sender.clone(),
            output_sender.clone(),
        );
        let leader_notify = Arc::new((Mutex::new(true), Condvar::new()));
        let leader_handler = LeaderHandler::new(
            leader_handler_receiver,
            peer_handler_sender.clone(),
            output_sender.clone(),
            leader_notify.clone(),
            self.id,
        );

        let lock = CentralizedLock::new();
        let lock_notify = Arc::new((Mutex::new(lock), Condvar::new()));
        let lock_handler = LockHandler::new(
            lock_handler_receiver,
            peer_handler_sender.clone(),
            lock_notify.clone(),
        );

        let connection_handler = ConnectionHandler::new(sender.clone(), port_from, port_to);
        let input_handler = InputHandler::new(source, sender, output_receiver);

        let message_handler = MessageHandler::new(
            self.id,
            message_handler_receiver,
            peer_handler_sender.clone(),
            leader_notify,
            leader_handler_sender.clone(),
            output_sender,
            lock_notify.clone(),
        );

        self.dispatch_messages(
            receiver,
            peer_handler_sender,
            message_handler_sender,
            leader_handler_sender,
            lock_handler_sender,
            lock_notify,
        )?;

        drop(connection_handler);
        drop(peer_handler);
        drop(input_handler);
        drop(message_handler);
        drop(leader_handler);
        drop(lock_handler);

        Ok(())
    }

    fn dispatch_messages(
        &mut self,
        event_receiver: Receiver<ClientEvent>,
        peer_sender: Sender<ClientEvent>,
        message_sender: Sender<(ClientMessage, PeerIdType)>,
        leader_sender: Sender<(LeaderMessage, PeerIdType)>,
        lock_sender: Sender<PeerIdType>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> io::Result<()> {
        while let Ok(event) = event_receiver.recv() {
            match event {
                ClientEvent::Connection { .. } | ClientEvent::PeerDisconnected { .. } => {
                    peer_sender
                        .send(event)
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "peer sender error"))?;
                }
                ClientEvent::PeerMessage { message, peer_id } => match message {
                    Message::Common(message) => {
                        message_sender.send((message, peer_id)).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "message sender error")
                        })?;
                    }
                    Message::Leader(message) => {
                        leader_sender.send((message, peer_id)).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "leader sender error")
                        })?;
                    }
                    Message::Lock(message) => match message {
                        LockMessage::Acquire => {
                            println!("acquire to {}", peer_id);
                            lock_sender.send(peer_id).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "lock sender error")
                            })?;
                        }
                        LockMessage::Release => {
                            println!("TODO: IMPLEMENTAR RELEASE");
                            println!("TODO: IMPLEMENTAR RELEASE");
                            println!("TODO: IMPLEMENTAR RELEASE");
                            println!("TODO: IMPLEMENTAR RELEASE");
                            println!("TODO: IMPLEMENTAR RELEASE");
                            let (_, cv) = lock_notify.deref();

                            cv.notify_all();
                        }
                    },
                },
                ClientEvent::UserInput { message } => match &message {
                    Message::Common(inner) => {
                        let leader_id = Client::retrieve_leader(&leader_sender);
                        if leader_id != self.id {
                            let event = ClientEvent::PeerMessage {
                                message,
                                peer_id: leader_id,
                            };
                            peer_sender.send(event).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "message sender error")
                            })?;
                        } else {
                            message_sender.send((inner.clone(), self.id)).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "message sender error")
                            })?;
                        }
                    }
                    Message::Leader(message) => {
                        leader_sender.send((message.clone(), 0)).map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "leader sender error")
                        })?;
                    }
                    Message::Lock(message) => {
                        let leader_id = Client::retrieve_leader(&leader_sender);
                        if leader_id == self.id {
                            lock_sender.send(self.id).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "lock sender error")
                            })?;
                        } else {
                            let event = ClientEvent::PeerMessage {
                                message: Message::Lock(message.clone()),
                                peer_id: leader_id,
                            };
                            peer_sender.send(event).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "peer sender error")
                            })?;
                        }
                    }
                },
            }
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
