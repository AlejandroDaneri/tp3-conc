use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::client_event::{ClientEvent, ClientMessage};
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputHandler;
use crate::handler::peer_handler::PeerHandler;

#[derive(Debug)]
pub struct Client {
    id: u32,
    lock: CentralizedLock,
    blockchain: Blockchain,
    leader: PeerIdType,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client {
            id,
            lock: CentralizedLock::new(), // tiene que saber cual es el lider
            blockchain: Blockchain::new(),
            leader: 0,
        }
    }
    // se llama cuando se lo designa coordinador
    fn set_coordinator(&mut self) {
        self.leader = self.id;
    }

    fn update_coordinator(&mut self, id: u32) {
        self.leader = id;
    }

    fn is_leader(&self) -> bool {
        self.id == self.leader
    }

    pub fn run(&mut self, port_from: u16, port_to: u16) -> io::Result<()> {
        let (sender, receiver) = channel();

        let connection_handler = ConnectionHandler::new(sender.clone(), port_from, port_to);
        let input_handler = InputHandler::new(sender.clone());

        let (peer_handler_sender, peer_handler_receiver) = channel();
        let peer_handler = PeerHandler::new(self.id, sender.clone(), peer_handler_receiver);

        self.dispatch_messages(receiver, peer_handler_sender);

        drop(connection_handler);
        drop(peer_handler);
        drop(input_handler);

        Ok(())
    }

    fn dispatch_messages(
        &mut self,
        event_receiver: Receiver<ClientEvent>,
        peer_sender: Sender<ClientEvent>,
    ) -> io::Result<()> {
        //proceso mensajes que me llegan
        while let Ok(event) = event_receiver.recv() {
            match event {
                ClientEvent::Connection { .. } | ClientEvent::PeerDisconnected { .. } => {
                    peer_sender.send(event);
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    if let Some(response) = self.process_message(message, peer_id) {
                        peer_sender.send(ClientEvent::PeerMessage {
                            peer_id,
                            message: response,
                        });
                    }
                }
                ClientEvent::UserInput { message } => {
                    // TODO ¿Poner un process_input más especializado? ¿Usar otro enum de mensajes?
                    self.process_message(message, self.id);
                }
            }
        }
        Ok(())
    }

    fn process_message(&mut self, message: ClientMessage, peer_id: u32) -> Option<ClientMessage> {
        println!("PROCESS: {:?}", message.serialize());
        match message {
            ClientMessage::ReadBlockchainRequest {} => {
                if !self.lock.is_owned_by(peer_id) {
                    return Some(ClientMessage::TodoMessage {
                        msg: "rb lock not acquired previosly".to_owned(),
                    });
                }
                if self.is_leader() {
                    Some(ClientMessage::ReadBlockchainResponse {
                        blockchain: self.blockchain.clone(),
                    })
                } else {
                    Some(ClientMessage::TodoMessage {
                        msg: "rb with no leader".to_owned(),
                    })
                }
            }
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                println!("Blockchain: {}", blockchain);
                None
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if self.is_leader() {
                    {
                        let _valid = self.blockchain.validate(transaction.clone()); //esto deberia ser la transaccion que recibe cuando devuelve el lock
                        self.blockchain.add_transaction(transaction);
                    }
                }
                Some(ClientMessage::TodoMessage { msg: format!("wb") })
            }

            ClientMessage::LockRequest { read_only: _ } => {
                // si me llega esto deberia ser lider
                // soy lider?
                if self.lock.acquire(peer_id) == LockResult::Acquired {
                    Some(ClientMessage::TodoMessage {
                        msg: format!("lock acquired"),
                    })
                } else {
                    Some(ClientMessage::TodoMessage {
                        msg: format!("lock failed"),
                    })
                }
            }

            ClientMessage::LeaderElectionRequest {
                request_id,
                timestamp: _,
            } => {
                //TODO: usar timestamp
                if request_id > self.id {
                    return Some(ClientMessage::TodoMessage {
                        msg: "Yo no puedo ser lider".to_owned(),
                    });
                }
                /*let leader = self.connected_peers.get(&(self.leader)).unwrap();
                let response = leader.write_message(ClientMessage::StillAlive {});
                if response.is_ok() {
                    return Some(ClientMessage::TodoMessage {
                        msg: format!("el lider sigue siendo: {}", self.leader),
                    });
                }
                //thread::spawn(move || Client::send_leader_request(self, self.id));
                */
                Some(ClientMessage::TodoMessage {
                    msg: "Bully OK".to_owned(),
                })
            }
            ClientMessage::OkMessage {} => None,

            ClientMessage::CoordinatorMessage { connection_id: id } => {
                self.update_coordinator(id);
                if self.leader != self.id {
                    println!("New leader: {}", id);
                }
                Some(ClientMessage::TodoMessage {
                    msg: format!("CoordinatorUpdate {}", id),
                })
            }
            ClientMessage::StillAlive {} => None,
            ClientMessage::TodoMessage { msg: _msg } => None,
        }
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
