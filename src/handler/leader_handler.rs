use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};
use std::{io, sync::mpsc::Receiver, thread};

use crate::blockchain::peer::PeerIdType;
use crate::communication::client_event::{ClientEvent, ClientMessage, LeaderMessage, Message};

const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct LeaderProcessor {
    peer_handler_sender: Sender<ClientEvent>,
    output_sender: Sender<ClientMessage>,
    current_leader: PeerIdType,
    own_id: u32,
    waiting_coordinator: bool,
    election_in_progress: bool,
    election_by_user: bool,
}

impl LeaderHandler {
    pub fn new(
        leader_receiver: Receiver<(LeaderMessage, PeerIdType)>,
        peer_handler_sender: Sender<ClientEvent>,
        output_sender: Sender<ClientMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
        own_id: u32,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(
                leader_receiver,
                peer_handler_sender,
                output_sender,
                leader_election_notify,
                own_id,
            )
            .unwrap();
        }));
        LeaderHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<(LeaderMessage, PeerIdType)>,
        peer_handler_sender: Sender<ClientEvent>,
        output_sender: Sender<ClientMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
        own_id: u32,
    ) -> io::Result<()> {
        let mut processor = LeaderProcessor::new(own_id, peer_handler_sender, output_sender);
        processor.run(message_receiver, leader_election_notify)
    }
}

impl LeaderProcessor {
    pub fn new(
        own_id: u32,
        peer_handler_sender: Sender<ClientEvent>,
        output_sender: Sender<ClientMessage>,
    ) -> Self {
        LeaderProcessor {
            current_leader: 0,
            peer_handler_sender,
            output_sender,
            own_id,
            waiting_coordinator: false,
            election_in_progress: false,
            election_by_user: false,
        }
    }

    fn notify_victory(&self) {
        let message = Message::Leader(LeaderMessage::VictoryMessage);
        self.peer_handler_sender
            .send(ClientEvent::PeerMessage {
                message,
                peer_id: self.own_id,
            })
            .ok();
    }
    pub fn run(
        &mut self,
        receiver: Receiver<(LeaderMessage, PeerIdType)>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        loop {
            match receiver.recv_timeout(LEADER_ELECTION_TIMEOUT) {
                Ok((message, peer_id)) => {
                    let (mutex, cv) = &*leader_election_notify;
                    if let Ok(mut leader_busy) = mutex.lock() {
                        debug!("Leader message from {}: {:?}", peer_id, message);
                        self.process_message(message, peer_id);
                        *leader_busy = true;
                    }
                    cv.notify_all();
                }
                Err(RecvTimeoutError::Timeout) => {
                    let (mutex, cv) = &*leader_election_notify;
                    // Si había una elección, se termina
                    if self.election_in_progress {
                        self.election_in_progress = false;
                        // Ningún mayor me dijo Ok
                        if !self.waiting_coordinator {
                            self.notify_victory();
                            self.current_leader = self.own_id;
                            if self.election_by_user {
                                self.output_sender
                                    .send(ClientMessage::LeaderElectionFinished)
                                    .unwrap();
                                self.election_by_user = false;
                            }
                        }
                    }
                    if let Ok(mut leader_busy) = mutex.lock() {
                        *leader_busy = false;
                    }
                    cv.notify_all();
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if self.election_in_progress && !self.waiting_coordinator {
                        // send coordinatortoall
                        self.election_in_progress = false;
                        let (_, cv) = &*leader_election_notify;
                        cv.notify_all();
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn process_message(&mut self, message: LeaderMessage, peer_id: PeerIdType) {
        match message {
            // Un proceso de pid menor quiere ser lider
            LeaderMessage::LeaderElectionRequest { .. } => {
                self.run_election(message, peer_id);
            }
            // Alguien de pid mayor me dijo "Ok", así que espero el victory
            LeaderMessage::OkMessage => self.waiting_coordinator = true,
            // Alguien de pid mayor salió lider electo democráticamente, todos amamos al lider
            LeaderMessage::VictoryMessage {} => {
                info!(
                    "new leader: {}, initiated by me: {}",
                    peer_id, self.election_by_user
                );
                if self.election_by_user {
                    self.output_sender
                        .send(ClientMessage::LeaderElectionFinished)
                        .unwrap();
                    self.election_by_user = false;
                }
                self.current_leader = peer_id;
                self.waiting_coordinator = false;
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                debug!("Current leader: {}", self.current_leader);
                response_sender.send(self.current_leader).unwrap();
            }
            LeaderMessage::PeerDisconnected => {
                if peer_id == self.current_leader && self.own_id != peer_id {
                    self.run_election(message, peer_id);
                }
            }
            LeaderMessage::SendLeaderId => {
                if self.current_leader == self.own_id {
                    self.peer_handler_sender
                        .send(ClientEvent::PeerMessage {
                            message: Message::Leader(LeaderMessage::VictoryMessage),
                            peer_id,
                        })
                        .ok();
                }
            }
            LeaderMessage::BroadcastBlockchain { blockchain } => {
                self.peer_handler_sender
                    .send(ClientEvent::PeerMessage {
                        peer_id: self.own_id,
                        message: Message::Common(ClientMessage::BroadcastBlockchain { blockchain }),
                    })
                    .ok();
            }
        }
    }

    fn run_election(&mut self, _message: LeaderMessage, peer_id: PeerIdType) {
        self.election_in_progress = true;
        // Viene desde un comando de usuario
        info!("Election by {}", peer_id);
        if peer_id == 0 {
            let message = Message::Leader(LeaderMessage::LeaderElectionRequest {
                timestamp: SystemTime::now(),
            });
            self.election_by_user = true;
            self.peer_handler_sender
                .send(ClientEvent::PeerMessage { message, peer_id })
                .ok();
        } else {
            let message = Message::Leader(LeaderMessage::OkMessage);
            debug!("Mando a peer {:?}", message);
            self.peer_handler_sender
                .send(ClientEvent::PeerMessage { message, peer_id })
                .ok();
            let message = Message::Leader(LeaderMessage::LeaderElectionRequest {
                timestamp: SystemTime::now(),
            });
            debug!("Mando LE");
            self.peer_handler_sender
                .send(ClientEvent::PeerMessage { message, peer_id })
                .ok();
        };
    }
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
