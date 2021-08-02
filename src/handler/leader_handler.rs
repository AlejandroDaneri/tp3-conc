use std::{io, sync::mpsc::Receiver, thread, time::SystemTime};

use crate::blockchain::{client_event::LeaderMessage, peer::PeerIdType};

use super::peer_handler::PeerHandler;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use std::sync::mpsc::RecvTimeoutError;

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(2);

struct LeaderProcessor {
    peer_handler: PeerHandler,
    current_leader: PeerIdType,
}

impl LeaderHandler {
    pub fn new(
        leader_receiver: Receiver<LeaderMessage>,
        peer_handler: PeerHandler,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(leader_receiver, peer_handler, leader_election_notify).unwrap();
        }));
        LeaderHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<LeaderMessage>,
        peer_handler: PeerHandler,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        let mut processor = LeaderProcessor::new(peer_handler);
        processor.leader_processor(message_receiver, leader_election_notify)
    }
}

impl LeaderProcessor {
    pub fn new(peer_handler: PeerHandler) -> Self {
        LeaderProcessor {
            peer_handler,
            current_leader: 0,
        }
    }

    fn notify_all(&self, _id: u32) {
        println!("----notify----");

        for (peer_pid, peer) in self.peer_handler.connected_peers.iter() {
            peer.write_message_leader(LeaderMessage::CoordinatorMessage {
                connection_id: self.peer_handler.own_id,
            });
        }
    }
    pub fn leader_processor(
        &mut self,
        receiver: Receiver<LeaderMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        loop {
            match receiver.recv_timeout(LEADER_ELECTION_TIMEOUT) {
                Ok(message) => {
                    let (mutex, cv) = &*leader_election_notify;
                    if let Ok(mut leader_busy) = mutex.lock() {
                        self.process_message(message);
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
                    let (_, cv) = &*leader_election_notify;
                    cv.notify_all();
                    break;
                }
            }
        }
        Ok(())
    }

    fn process_message(&mut self, message: LeaderMessage) {
        match message {
            LeaderMessage::LeaderElectionRequest {
                request_id,
                timestamp: _,
            } => {
                //TODO: usar timestamp
                if request_id > self.peer_handler.own_id {
                    return; //no puedo ser lider, descartado
                }
                let asker = self
                    .peer_handler
                    .connected_peers
                    .get(&(&request_id))
                    .unwrap();
                asker.write_message_leader(LeaderMessage::OkMessage {});
                self.send_leader_request();
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                response_sender.send(self.current_leader).unwrap();
            }
            LeaderMessage::CoordinatorMessage { connection_id } => {
                self.current_leader = connection_id
            }
            LeaderMessage::StillAlive {} => todo!(),
            LeaderMessage::TodoMessage { msg: _ } => todo!(),
            LeaderMessage::OkMessage => todo!(),
        }
    }

    fn send_leader_request(&self) {
        let mut peer_handler = self.peer_handler;
        println!("MANDE LIDER");
        let mut higher_alive = false;
        for (peer_pid, peer) in peer_handler.connected_peers.iter() {
            if peer_pid > &(peer_handler.own_id) {
                println!("hay peer que puede ser lider");
                let response = peer.write_message_leader(LeaderMessage::LeaderElectionRequest {
                    request_id: peer_handler.own_id,
                    timestamp: SystemTime::now(),
                });

                // TODO: necesito que alguno me responda por algun canal para saber que no voy a ser lider
                if response.is_ok() {
                    higher_alive = true
                }
            }
        }

        if higher_alive {
            println!("NO SOY NUEVO LIDER");
            return;
        }
        self.current_leader = peer_handler.own_id;
        println!("SOY NUEVO LIDER");
        self.notify_all(peer_handler.own_id);
    }
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
