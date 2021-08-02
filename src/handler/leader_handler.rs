use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use std::{io, sync::mpsc::Receiver, thread};

use crate::blockchain::client_event::{ClientMessage, LeaderMessage};
use crate::blockchain::peer::PeerIdType;
use std::sync::mpsc::RecvTimeoutError;

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(2);

struct LeaderProcessor {
    current_leader: PeerIdType,
}

impl LeaderHandler {
    pub fn new(
        leader_receiver: Receiver<LeaderMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(leader_receiver, leader_election_notify).unwrap();
        }));
        LeaderHandler { thread_handle }
    }

    fn run(
        message_receiver: Receiver<LeaderMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        let mut processor = LeaderProcessor::new();
        // for (message, peer_id) in message_receiver {
        /*if let Some(response) =*/
        processor.leader_processor(message_receiver, leader_election_notify)
        // peer_sender.send(ClientEvent::LeaderMessage {
        //     peer_id,
        //     message: response,
        // });

        // Ok(())
    }
    // println!("Saliendo del hilo de mensajes");
    // Ok(())
}

impl LeaderProcessor {
    pub fn new() -> Self {
        LeaderProcessor { current_leader: 0 }
    }

    fn notify_all(&self, _id: u32) {
        println!("----notify----");
        /*
        Crear un evento que pueda ser enviado al peer_handler

        for (peer_pid, peer) in self.connected_peers.iter() {

            peer.write_message(ClientMessage::CoordinatorMessage {
                connection_id: self.id,
            });
        }*/
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

    fn process_message(&mut self, message: LeaderMessage) -> io::Result<()> {
        match message {
            LeaderMessage::LeaderElectionRequest {
                request_id,
                timestamp: _,
            } => {
                self.current_leader = request_id;
                /*
                //TODO: usar timestamp
                if request_id > self.id {
                    return Some(LeaderMessage::TodoMessage {
                        msg: "Yo no puedo ser lider".to_owned(),
                    });
                }
                let leader = self.connected_peers.get(&(self.leader)).unwrap();
                let response = leader.write_message(ClientMessage::StillAlive {});
                if response.is_ok() {
                    return Some(ClientMessage::TodoMessage {
                        msg: format!("el lider sigue siendo: {}", self.leader),
                    });
                }
                //thread::spawn(move || Client::send_leader_request(self, self.id));
                */
                // Some(LeaderMessage::TodoMessage {
                //     msg: "Bully OK".to_owned(),
                // })
            }
            LeaderMessage::CurrentLeaderLocal { response_sender } => {
                response_sender.send(self.current_leader).unwrap();
            }
            LeaderMessage::CoordinatorMessage { connection_id: _ } => todo!(),
            LeaderMessage::StillAlive {} => todo!(),
            LeaderMessage::TodoMessage { msg: _ } => todo!(),
            LeaderMessage::OkMessage => todo!(),
        }
        Ok(())
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
}

impl Drop for LeaderHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
