use std::{io, sync::mpsc::Receiver, thread, time::SystemTime};

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
}

impl LeaderHandler {
    pub fn new(
        leader_receiver: Receiver<LeaderMessage>,
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
        message_receiver: Receiver<LeaderMessage>,
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
        }
    }

    fn notify_all(&self, _id: u32) {
        println!("----notify----");
        self.peer_handler_sender
            .send(ClientEvent::CoordinatorToAll {});
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
                        // TODO: esperar que llegue coordinador, procesar y ahi si desbloquear
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
                if request_id > self.own_id {
                    return; //no puedo ser lider, descartado
                }

                self.peer_handler_sender.send(ClientEvent::SendOkTo {
                    destination_id: request_id,
                });
                self.send_leader_request();
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                response_sender.send(self.current_leader).unwrap();
            }
            LeaderMessage::CoordinatorMessage { connection_id } => {
                self.current_leader = connection_id
            }
            LeaderMessage::StillAlive {} => {}
            LeaderMessage::OkMessage => {}
        }
    }

    fn send_leader_request(&mut self) {
        println!("MANDE LIDER");
        let mut higher_alive = false;
        for (peer_pid, peer) in self.peer_handler.connected_peers.iter() {
            if peer_pid > &(self.own_id) {
                println!("hay peer que puede ser lider");
                let response = peer.write_message_leader(LeaderMessage::LeaderElectionRequest {
                    request_id: self.own_id,
                    timestamp: SystemTime::now(),
                });

                // TODO: necesito que alguno me responda por algun canal el OKMEssage para saber que no voy a ser lider
                if response.is_ok() {
                    higher_alive = true
                }
            }
        }

        if higher_alive {
            println!("NO SOY NUEVO LIDER");
            return;
        }
        self.current_leader = self.own_id;
        println!("SOY NUEVO LIDER");
        self.notify_all(self.own_id);
    }
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
