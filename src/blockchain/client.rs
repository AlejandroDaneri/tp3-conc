use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::blockchain::client_event::{ClientEvent, ClientMessage};
use crate::blockchain::peer::PeerIdType;
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputHandler;
use crate::handler::leader_handler::LeaderHandler;
use crate::handler::message_handler::MessageHandler;
use crate::handler::peer_handler::PeerHandler;

use super::client_event::LeaderMessage;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub struct Client {
    id: u32,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client { id }
    }

    pub fn run(&mut self, port_from: u16, port_to: u16) -> io::Result<()> {
        let (sender, receiver) = channel();

        let connection_handler = ConnectionHandler::new(sender.clone(), port_from, port_to);
        let input_handler = InputHandler::new(sender.clone());

        let (peer_handler_sender, peer_handler_receiver) = channel();
        let peer_handler = PeerHandler::new(self.id, sender, peer_handler_receiver);

        let (leader_handler_sender, leader_handler_receiver) = channel();

        let leader_notify = Arc::new((Mutex::new(true), Condvar::new()));
        let leader_handler =
            LeaderHandler::new(leader_handler_receiver, peer_handler, leader_notify.clone());

        let (message_handler_sender, message_handler_receiver) = channel();
        let message_handler = MessageHandler::new(
            message_handler_receiver,
            peer_handler_sender.clone(),
            leader_notify,
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
        leader_sender: Sender<LeaderMessage>,
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
                    // TODO ¿Poner un process_input más especializado? ¿Usar otro enum de mensajes?
                    message_sender.send((message, 0)).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "message sender error")
                    })?;
                }
                ClientEvent::LeaderEvent { message } => {
                    //parar todo llego un mensaje lider
                    leader_sender
                        .send(message)
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "leader sender error"))?;
                }
            }
        }
        Ok(())
    }
}
