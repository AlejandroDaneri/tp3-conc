use std::{io, sync::mpsc::Receiver, thread};

use crate::blockchain::client_event::ClientEvent;
use crate::blockchain::{client_event::LeaderMessage, peer::PeerIdType};

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

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

    fn notify_victory(&self, peer_id: u32) {
        let message = LeaderMessage::VictoryMessage {};
        self.peer_handler_sender
            .send(ClientEvent::LeaderEvent { message, peer_id });
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
                        self.process_message(message, peer_id);
                        *leader_busy = true;
                    }
                    cv.notify_all();
                }
                Err(RecvTimeoutError::Timeout) => {
                    let (mutex, cv) = &*leader_election_notify;
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
            LeaderMessage::LeaderElectionRequest { timestamp } => {
                println!("Leader election: {}", peer_id);
                self.election_in_progress = true;
                if peer_id < self.own_id {
                    let message = LeaderMessage::LeaderElectionRequest { timestamp };
                    self.peer_handler_sender
                        .send(ClientEvent::LeaderEvent { message, peer_id });
                }
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                response_sender.send(self.current_leader).unwrap();
            }
            LeaderMessage::VictoryMessage {} => {
                println!("Victory from: {}", peer_id);
                self.current_leader = peer_id
            }
            LeaderMessage::OkMessage => self.waiting_coordinator = true,
        }
    }
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
