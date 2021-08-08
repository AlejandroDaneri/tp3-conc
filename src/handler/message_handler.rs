use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::client_event::{
    ClientEvent, ClientMessage, ErrorMessage, LeaderMessage, Message,
};
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::PeerIdType;
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
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            MessageHandler::run(
                own_id,
                message_receiver,
                peer_sender,
                leader_notify,
                leader_handler_sender,
                output_sender,
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
    ) -> io::Result<()> {
        let mut processor = MessageProcessor::new(own_id, leader_handler_sender, output_sender);
        for (message, peer_id) in message_receiver {
            let (mutex, cv) = &*leader_notify;
            if let Ok(leader_lock) = mutex.lock() {
                let _guard = cv
                    .wait_while(leader_lock, |leader_busy| *leader_busy)
                    .unwrap();
            }
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
}

struct MessageProcessor {
    id: PeerIdType,
    lock: CentralizedLock,
    blockchain: Blockchain,
    leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
    output_sender: Sender<ClientMessage>,
}

impl MessageProcessor {
    pub fn new(
        own_id: PeerIdType,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
    ) -> Self {
        MessageProcessor {
            id: own_id,
            lock: CentralizedLock::new(),
            blockchain: Blockchain::new(),
            leader_handler_sender,
            output_sender,
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
                if !self.lock.is_owned_by(peer_id) {
                    return Some(ClientMessage::ErrorResponse(
                        ErrorMessage::LockNotAcquiredError,
                    ));
                }
                if self.is_leader() {
                    Some(ClientMessage::ReadBlockchainResponse {
                        blockchain: self.blockchain.clone(),
                    })
                } else {
                    Some(ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError))
                }
            }
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                self.blockchain = blockchain;
                self.output_sender.send(redirect);
                None
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if self.is_leader() {
                    {
                        let _valid = self.blockchain.validate(&transaction); //esto deberia ser la transaccion que recibe cuando devuelve el lock
                        self.blockchain.add_transaction(transaction);
                    }
                }
                Some(ClientMessage::WriteBlockchainResponse {})
            }
            ClientMessage::WriteBlockchainResponse {} => {
                self.output_sender.send(message);
                None
            }

            ClientMessage::LockRequest { read_only: _ } => {
                if self.is_leader() {
                    let acquired = self.lock.acquire(peer_id) == LockResult::Acquired;
                    Some(ClientMessage::LockResponse { acquired })
                } else {
                    Some(ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError))
                }
            }
            ClientMessage::LockResponse { .. } => {
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
