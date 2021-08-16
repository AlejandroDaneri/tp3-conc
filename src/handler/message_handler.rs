use std::io;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Condvar, Mutex};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::lock::Lock;
use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{
    ClientEvent, ClientMessage, ErrorMessage, LeaderMessage, Message,
};
use crate::communication::dispatcher::Dispatcher;
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
        dispatcher: Dispatcher,
        leader_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            MessageHandler::run(own_id, message_receiver, dispatcher, leader_notify).unwrap();
        }));
        MessageHandler { thread_handle }
    }

    fn run(
        own_id: PeerIdType,
        message_receiver: Receiver<(ClientMessage, PeerIdType)>,
        dispatcher: Dispatcher,
        leader_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        let mut processor = MessageProcessor::new(own_id, dispatcher.clone());
        for (message, peer_id) in message_receiver {
            MessageHandler::wait_leader_election(&leader_notify);
            if let Some(response) = processor.process_message(message, peer_id) {
                info!("Sending response to {}: {:?}", peer_id, response);
                dispatcher
                    .peer_sender
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
        warn!("Saliendo del hilo de mensajes");
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
    dispatcher: Dispatcher,
}

impl MessageProcessor {
    pub fn new(own_id: PeerIdType, dispatcher: Dispatcher) -> Self {
        MessageProcessor {
            id: own_id,
            blockchain: Blockchain::new(),
            dispatcher,
        }
    }

    pub fn process_message(
        &mut self,
        message: ClientMessage,
        peer_id: u32,
    ) -> Option<ClientMessage> {
        let redirect = message.clone();
        debug!("Processing: {:?}", message);
        match message {
            ClientMessage::ReadBlockchainRequest {} => {
                Some(ClientMessage::ReadBlockchainResponse {
                    blockchain: self.blockchain.clone(),
                })
            }
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                self.blockchain = blockchain;
                self.dispatcher.output_sender.send(redirect).ok()?;
                None
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if let Ok(guard) = self.dispatcher.lock_notify.0.lock() {
                    let owned = guard.is_owned_by(peer_id);
                    println!("WriteBlockchain... lock owned? {}", owned);
                    if !owned {
                        return Some(ClientMessage::ErrorResponse(
                            ErrorMessage::LockNotAcquiredError,
                        ));
                    }
                }
                if self.is_leader() {
                    let _valid = self.blockchain.validate(&transaction);
                    self.blockchain.add_transaction(transaction.clone());
                    self.dispatcher
                        .leader_sender
                        .send((
                            LeaderMessage::BroadcastBlockchain {
                                blockchain: self.blockchain.clone(),
                            },
                            self.id,
                        ))
                        .ok()?;
                    Some(ClientMessage::WriteBlockchainResponse { transaction })
                } else {
                    Some(ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError))
                }
            }
            ClientMessage::WriteBlockchainResponse { transaction } => {
                self.blockchain.add_transaction(transaction);
                self.dispatcher.output_sender.send(redirect).ok()?;
                None
            }
            ClientMessage::LockResponse { .. } => {
                println!("Obtuve lock? {:?}", message);
                self.dispatcher.output_sender.send(message).ok()?;
                None
            }
            ClientMessage::ErrorResponse(..) => {
                self.dispatcher.output_sender.send(message).ok()?;
                None
            }
            ClientMessage::LeaderElectionFinished {} => {
                self.dispatcher.output_sender.send(message).ok()?;
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
        self.dispatcher.leader_sender.send((message, 0)).unwrap();
        response_receiver.recv().unwrap()
    }
}

impl Drop for MessageHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
