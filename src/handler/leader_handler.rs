use std::{io, sync::mpsc::Receiver, thread};

use crate::blockchain::client_event::ClientEvent;
use crate::blockchain::{client_event::LeaderMessage, peer::PeerIdType};

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

use std::sync::mpsc::{RecvTimeoutError, Sender};

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(2);

struct LeaderProcessor {
    peer_handler_sender: Sender<ClientEvent>,
    current_leader: PeerIdType,
    own_id: u32,
    waiting_coordinator: bool,
    election_in_progress: bool,
}

impl LeaderHandler {
    pub fn new(
        leader_receiver: Receiver<(LeaderMessage, PeerIdType)>,
        peer_handler_sender: Sender<ClientEvent>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
        own_id: u32,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(
                leader_receiver,
                peer_handler_sender,
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
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
        own_id: u32,
    ) -> io::Result<()> {
        let mut processor = LeaderProcessor::new(peer_handler_sender, own_id);
        processor.leader_processor(message_receiver, leader_election_notify)
    }
}

impl LeaderProcessor {
    pub fn new(peer_handler_sender: Sender<ClientEvent>, own_id: u32) -> Self {
        LeaderProcessor {
            current_leader: 0,
            peer_handler_sender,
            own_id,
            waiting_coordinator: false,
            election_in_progress: false,
        }
    }

    fn notify_victory(&self) {
        let message = LeaderMessage::VictoryMessage {};
        self.peer_handler_sender.send(ClientEvent::LeaderEvent {
            message,
            peer_id: self.own_id,
        });
    }
    pub fn leader_processor(
        &mut self,
        receiver: Receiver<(LeaderMessage, PeerIdType)>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        loop {
            match receiver.recv_timeout(LEADER_ELECTION_TIMEOUT) {
                Ok((message, peer_id)) => {
                    let (mutex, cv) = &*leader_election_notify;
                    if let Ok(mut leader_busy) = mutex.lock() {
                        println!("Leader message from {}: {:?}", peer_id, message);
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
                self.election_in_progress = true;
                // Viene desde un comando de usuario
                if peer_id == 0 {
                    let message = LeaderMessage::LeaderElectionRequest {
                        timestamp: SystemTime::now(),
                    };
                    println!("Soy 0 por la consola {:?}", message);
                    self.peer_handler_sender
                        .send(ClientEvent::LeaderEvent { message, peer_id });
                } else {
                    let message = LeaderMessage::OkMessage;
                    println!("Mando a peer {:?}", message);
                    self.peer_handler_sender
                        .send(ClientEvent::LeaderEvent { message, peer_id });
                    let message = LeaderMessage::LeaderElectionRequest {
                        timestamp: SystemTime::now(),
                    };
                    println!("Mando a LE");
                    self.peer_handler_sender
                        .send(ClientEvent::LeaderEvent { message, peer_id });
                };
            }
            // Alguien de pid mayor me dijo "Ok", así que espero el victory
            LeaderMessage::OkMessage => self.waiting_coordinator = true,
            // Alguien de pid mayor salió lider electo democráticamente, todos amamos al lider
            LeaderMessage::VictoryMessage {} => {
                println!("new leader: {}", peer_id);
                self.current_leader = peer_id
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                println!("Current leader: {}", self.current_leader);
                response_sender.send(self.current_leader).unwrap();
            }
        }
    }
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
