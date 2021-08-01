use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::client_event::{ClientEvent, ClientEventReader, ClientMessage};
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;

#[derive(Debug)]
pub struct MessageHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct MessageProcessor {
    id: PeerIdType,
    leader: PeerIdType,
    lock: CentralizedLock,
    blockchain: Blockchain,
}

impl MessageHandler {
    pub fn new(
        message_receiver: Receiver<(ClientMessage, PeerIdType)>,
        peer_sender: Sender<ClientEvent>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            MessageHandler::run(message_receiver, peer_sender).unwrap();
        }));
        MessageHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<(ClientMessage, PeerIdType)>,
        peer_sender: Sender<ClientEvent>,
    ) -> io::Result<()> {
        let mut processor = MessageProcessor::new();
        for (message, peer_id) in message_receiver {
            if let Some(response) = processor.process_message(message, peer_id) {
                peer_sender.send(ClientEvent::PeerMessage {
                    peer_id,
                    message: response,
                });
            }
        }
        println!("Saliendo del hilo de mensajes");
        Ok(())
    }
}

impl MessageProcessor {
    pub fn new() -> Self {
        MessageProcessor {
            id: 0,
            leader: 0,
            lock: CentralizedLock::new(),
            blockchain: Blockchain::new(),
        }
    }

    pub fn process_message(
        &mut self,
        message: ClientMessage,
        peer_id: u32,
    ) -> Option<ClientMessage> {
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
                //self.update_coordinator(id);
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

    fn is_leader(&self) -> bool {
        self.id == self.leader
    }
}

impl Drop for MessageHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}