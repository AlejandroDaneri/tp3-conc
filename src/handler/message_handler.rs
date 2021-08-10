use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::lock::{CentralizedLock, Lock};
use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{
    ClientEvent, ClientMessage, ErrorMessage, LeaderMessage, Message,
};
use std::ops::Deref;
use std::thread;

#[derive(Debug)]
pub struct MessageHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl MessageHandler {
    pub fn new(
        own_id: PeerIdType,
        message_receiver: Receiver<(ClientMessage, PeerIdType)>,
        peer_sender: Sender<ClientEvent>,
        leader_notify: Arc<(Mutex<bool>, Condvar)>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            MessageHandler::run(
                own_id,
                message_receiver,
                peer_sender,
                leader_notify,
                leader_handler_sender,
                output_sender,
                lock_notify,
            )
            .unwrap();
        }));
        MessageHandler { thread_handle }
    }

    fn run(
        own_id: PeerIdType,
        message_receiver: Receiver<(ClientMessage, PeerIdType)>,
        peer_sender: Sender<ClientEvent>,
        leader_notify: Arc<(Mutex<bool>, Condvar)>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> io::Result<()> {
        let mut processor =
            MessageProcessor::new(own_id, leader_handler_sender, output_sender, lock_notify);
        for (message, peer_id) in message_receiver {
            MessageHandler::wait_leader_election(&leader_notify);
            if let Some(response) = processor.process_message(message, peer_id) {
                peer_sender
                    .send(ClientEvent::PeerMessage {
                        message: Message::Common(response),
                        peer_id,
                    })
                    .map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            "[Process message] Error while sending message to peer",
                        )
                    })?;
            }
        }
        println!("Saliendo del hilo de mensajes");
        Ok(())
    }

    fn wait_leader_election(leader_notify: &Arc<(Mutex<bool>, Condvar)>) {
        let (mutex, cv) = leader_notify.deref();
        if let Ok(leader_lock) = mutex.lock() {
            let _guard = cv
                .wait_while(leader_lock, |leader_busy| *leader_busy)
                .unwrap();
        }
    }
}

struct MessageProcessor {
    id: PeerIdType,
    blockchain: Blockchain,
    leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
    output_sender: Sender<ClientMessage>,
    lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
}

impl MessageProcessor {
    pub fn new(
        own_id: PeerIdType,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
        lock_notify: Arc<(Mutex<CentralizedLock>, Condvar)>,
    ) -> Self {
        MessageProcessor {
            id: own_id,
            blockchain: Blockchain::new(),
            leader_handler_sender,
            output_sender,
            lock_notify,
        }
    }

    pub fn process_message(
        &mut self,
        message: ClientMessage,
        peer_id: u32,
    ) -> Option<ClientMessage> {
        let redirect = message.clone();
        println!("process_message: {:?}", message);
        match message {
            ClientMessage::ReadBlockchainRequest {} => {
                Some(ClientMessage::ReadBlockchainResponse {
                    blockchain: self.blockchain.clone(),
                })
            }
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                self.blockchain = blockchain;
                self.output_sender.send(redirect);
                None
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if let Ok(guard) = self.lock_notify.0.lock() {
                    let locked = guard.is_owned_by(peer_id);
                    if !locked {
                        return Some(ClientMessage::ErrorResponse(
                            ErrorMessage::LockNotAcquiredError,
                        ));
                    }
                }
                if self.is_leader() {
                    let _valid = self.blockchain.validate(&transaction); //esto deberia ser la transaccion que recibe cuando devuelve el lock
                    self.blockchain.add_transaction(transaction);
                    self.leader_handler_sender.send((
                        LeaderMessage::BroadcastBlockchain {
                            blockchain: self.blockchain.clone(),
                        },
                        self.id,
                    ));
                    Some(ClientMessage::WriteBlockchainResponse {})
                } else {
                    Some(ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError))
                }
            }
            ClientMessage::WriteBlockchainResponse {} => {
                self.output_sender.send(message);
                None
            }
            ClientMessage::LockResponse { .. } => {
                print!("Obtuve lock? {:?}", message);
                self.output_sender.send(message);
                None
            }
            ClientMessage::ErrorResponse(..) => {
                self.output_sender.send(message);
                None
            }
            ClientMessage::LeaderElectionFinished {} => {
                self.output_sender.send(message);
                None
            }
            ClientMessage::BroadcastBlockchain { blockchain } => {
                self.blockchain = blockchain;
                None
            }
        }
    }

    fn is_leader(&self) -> bool {
        self.id == self.retrieve_leader()
    }

    fn retrieve_leader(&self) -> PeerIdType {
        let (response_sender, response_receiver) = channel();
        let message = LeaderMessage::CurrentLeaderLocal { response_sender };
        self.leader_handler_sender.send((message, 0)).unwrap();
        response_receiver.recv().unwrap()
    }
}

impl Drop for MessageHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
