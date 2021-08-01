use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::client_event::{ClientEvent, ClientMessage};
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputHandler;
use crate::handler::message_handler::MessageHandler;
use crate::handler::peer_handler::PeerHandler;

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
        let peer_handler = PeerHandler::new(self.id, sender.clone(), peer_handler_receiver);

        let (message_handler_sender, message_handler_receiver) = channel();
        let message_handler =
            MessageHandler::new(message_handler_receiver, peer_handler_sender.clone());

        self.dispatch_messages(receiver, peer_handler_sender, message_handler_sender);

        drop(connection_handler);
        drop(peer_handler);
        drop(input_handler);

        Ok(())
    }

    fn dispatch_messages(
        &mut self,
        event_receiver: Receiver<ClientEvent>,
        peer_sender: Sender<ClientEvent>,
        message_sender: Sender<(ClientMessage, PeerIdType)>,
    ) -> io::Result<()> {
        //proceso mensajes que me llegan
        while let Ok(event) = event_receiver.recv() {
            match event {
                ClientEvent::Connection { .. } | ClientEvent::PeerDisconnected { .. } => {
                    peer_sender.send(event);
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    message_sender.send((message, peer_id));
                }
                ClientEvent::UserInput { message } => {
                    // TODO ¿Poner un process_input más especializado? ¿Usar otro enum de mensajes?
                    message_sender.send((message, 0));
                }
            }
        }
        Ok(())
    }

    fn send_result(&mut self, _id: u32, _result: u32) {}
    fn send_modifications(&mut self, _id: u32, _result: u32) {}

    fn notify_minions(&self, _id: u32) {
        println!("----notify----");
        /*
        Crear un evento que pueda ser enviado al peer_handler

        for (peer_pid, peer) in self.connected_peers.iter() {

            peer.write_message(ClientMessage::CoordinatorMessage {
                connection_id: self.id,
            });
        }*/
    }

    fn send_leader_request(&mut self, _id: u32) {
        println!("MANDE LIDER");
        /*let mut higher_alive = false;
        for (peer_pid, peer) in self.connected_peers.iter() {
            if peer_pid > &(self.id) {
                println!("hay peer que pueden ser lider");
                let response = peer.write_message(ClientMessage::LeaderElectionRequest {
                    request_id: self.id,
                    timestamp: SystemTime::now(),
                });
                if response.is_ok() {
                    higher_alive = true
                }
            }
        }

        if higher_alive {
            println!("NO SOY NUEVO LIDER");
            return;
        }
        self.leader = self.id;
        println!("SOY NUEVO LIDER");
        self.notify_minions(self.id);
        */
    }

    fn send_request_to_leader(&self, _message: ClientMessage) -> io::Result<()> {
        /*if let Some(leader_peer) = self.connected_peers.get(&self.leader) {
            leader_peer.write_message(message)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Request sent to none leader",
            ))
        }*/
        unimplemented!()
    }

    fn get_leader_id(&mut self, _id: u32, _result: u32) -> u32 {
        0
    }
}
