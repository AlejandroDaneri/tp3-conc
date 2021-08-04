use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::blockchain::client_event::{ClientEvent, ClientMessage, Message};
use crate::blockchain::peer::PeerIdType;
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputHandler;
use crate::handler::leader_handler::LeaderHandler;
use crate::handler::message_handler::MessageHandler;
use crate::handler::peer_handler::PeerHandler;

use super::client_event::LeaderMessage;
use std::io::Read;
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
        let (peer_handler_sender, peer_handler_receiver) = channel();
        let (message_handler_sender, message_handler_receiver) = channel();
        let peer_handler = PeerHandler::new(
            self.id,
            peer_handler_receiver,
            sender.clone(),
            leader_handler_sender.clone(),
        );
        let leader_notify = Arc::new((Mutex::new(true), Condvar::new()));
        let leader_handler = LeaderHandler::new(
            leader_handler_receiver,
            peer_handler_sender.clone(),
            leader_notify.clone(),
            self.id,
        );

        let connection_handler = ConnectionHandler::new(sender.clone(), port_from, port_to);
        let input_handler = InputHandler::new(source, sender);

        let message_handler = MessageHandler::new(
            self.id,
            message_handler_receiver,
            peer_handler_sender.clone(),
            leader_notify,
            leader_handler_sender.clone(),
        );

        self.dispatch_messages(
            receiver,
            peer_handler_sender,
            leader_handler_sender,
            message_handler_sender,
        )?;

        drop(connection_handler);
        drop(peer_handler);
        drop(input_handler);
        drop(message_handler);
        drop(leader_handler);

        Ok(())
    }

    fn dispatch_messages(
        &mut self,
        event_receiver: Receiver<ClientEvent>,
        peer_sender: Sender<ClientEvent>,
        leader_sender: Sender<(LeaderMessage, PeerIdType)>,
        message_sender: Sender<(ClientMessage, PeerIdType)>,
    ) -> io::Result<()> {
        while let Ok(event) = event_receiver.recv() {
            match event {
                ClientEvent::Connection { .. } | ClientEvent::PeerDisconnected { .. } => {
                    peer_sender
                        .send(event)
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "peer sender error"))?;
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    message_sender.send((message, peer_id)).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "message sender error")
                    })?;
                }
                ClientEvent::UserInput { message } => {
                    match message {
                        Message::Common(message) => {
                            let leader_id = Client::retrieve_leader(&leader_sender);
                            let event = ClientEvent::PeerMessage {
                                message,
                                peer_id: leader_id,
                            };
                            peer_sender.send(event).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "message sender error")
                            })?;
                        }
                        Message::Leader(message) => {
                            leader_sender.send((message, 0)).map_err(|_| {
                                io::Error::new(io::ErrorKind::Other, "message sender error")
                            })?;
                        }
                    }
                    // TODO ¿Poner un process_input más especializado? ¿Usar otro enum de mensajes?
                }
                ClientEvent::LeaderEvent { message, peer_id } => {
                    //parar todo llego un mensaje lider
                    leader_sender
                        .send((message, peer_id))
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "leader sender error"))?;
                }
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
