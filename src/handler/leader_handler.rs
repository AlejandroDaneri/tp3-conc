use std::{io, sync::mpsc::Receiver, thread, time::SystemTime};

use crate::blockchain::{
    client_event::{ClientMessage, LeaderMessage},
    peer::PeerIdType,
};

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
    actual_leader: PeerIdType,
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
        let processor = LeaderProcessor::new(peer_handler);
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
    pub fn new(peer_handler: PeerHandler) -> Self {
        LeaderProcessor {
            peer_handler,
            actual_leader: 0,
        }
    }

    fn notify_all(&self, _id: u32) {
        println!("----notify----");

        for (peer_pid, peer) in self.peer_handler.connected_peers.iter() {
            peer.write_message(LeaderMessage::CoordinatorMessage {
                connection_id: self.peer_handler.own_id,
            });
        }
    }
    // TODO: arreglar conflicto con la def de abajo
    // pub fn leader_processor(&self, receiver: Receiver<LeaderMessage>) {
    //     while let Ok(message) = receiver.recv() {
    //         match message {
    //             LeaderMessage::LeaderElectionRequest {
    //                 request_id,
    //                 timestamp: _,
    //             } => {
    //                 //TODO: usar timestamp
    //                 if request_id > self.peer_handler.own_id {
    //                     return;
    //                 }
    //                 let guard = thread::spawn(move || {
    //                     LeaderHandler::send_leader_request(self.peer_handler)
    //                 });

    //                 let asker = self
    //                     .peer_handler
    //                     .connected_peers
    //                     .get(&(&request_id))
    //                     .unwrap();
    //                 asker.write_message(LeaderMessage::OkMessage {});
    //                 guard.join()
    //             }
    //             LeaderMessage::CoordinatorMessage { connection_id } => {
    //                 self.actual_leader = connection_id
    //             }
    //             LeaderMessage::StillAlive {} => todo!(),
    //             LeaderMessage::TodoMessage { msg: _ } => todo!(),
    //             LeaderMessage::OkMessage => {}
    //         }
    //     }
    // }
    pub fn leader_processor(
        &self,
        receiver: Receiver<LeaderMessage>,
        leader_election_notify: Arc<(Mutex<bool>, Condvar)>,
    ) -> io::Result<()> {
        loop {
            match receiver.recv_timeout(LEADER_ELECTION_TIMEOUT) {
                Ok(message) => {
                    let (mutex, cv) = &*leader_election_notify;
                    if let Ok(mut leader_ready) = mutex.lock() {
                        *leader_ready = false;
                    }
                    cv.notify_all();
                }
                Err(RecvTimeoutError::Timeout) => {
                    println!("Leader election finished!");
                    let (mutex, cv) = &*leader_election_notify;
                    if let Ok(mut leader_ready) = mutex.lock() {
                        *leader_ready = true;
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

    fn process_message(&self, message: LeaderMessage) -> io::Result<()> {
        match message {
            LeaderMessage::LeaderElectionRequest {
                request_id: _,
                timestamp: _,
            } => {
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
            LeaderMessage::CoordinatorMessage { connection_id: _ } => todo!(),
            LeaderMessage::StillAlive {} => todo!(),
            LeaderMessage::TodoMessage { msg: _ } => todo!(),
            LeaderMessage::OkMessage => todo!(),
        }
        Ok(())
    }

    pub fn send_leader_request(&self, peer_handler: PeerHandler) {
        println!("MANDE LIDER");
        let mut higher_alive = false;
        for (peer_pid, peer) in peer_handler.connected_peers.iter() {
            if peer_pid > &(peer_handler.own_id) {
                println!("hay peer que pueden ser lider");
                let response = peer.write_message(LeaderMessage::LeaderElectionRequest {
                    request_id: peer_handler.own_id,
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
        self.actual_leader = peer_handler.own_id;
        println!("SOY NUEVO LIDER");
        self.notify_all(peer_handler.own_id);
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
