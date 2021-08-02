use std::{io, sync::mpsc::Receiver, thread, time::SystemTime};

use crate::blockchain::{
    client_event::{ClientMessage, LeaderMessage},
    peer::PeerIdType,
};

use super::peer_handler::PeerHandler;

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct LeaderProcessor {
    peer_handler: PeerHandler,
    actual_leader: PeerIdType,
}

impl LeaderHandler {
    pub fn new(leader_receiver: Receiver<LeaderMessage>, peer_handler: PeerHandler) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(leader_receiver, peer_handler).unwrap();
        }));
        LeaderHandler { thread_handle }
    }

    fn run(message_receiver: Receiver<LeaderMessage>, peer_handler: PeerHandler) -> io::Result<()> {
        let processor = LeaderProcessor::new(peer_handler);
        // for (message, peer_id) in message_receiver {
        /*if let Some(response) =*/
        Ok(processor.leader_processor(message_receiver))
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

    pub fn leader_processor(&self, receiver: Receiver<LeaderMessage>) {
        while let Ok(message) = receiver.recv() {
            match message {
                LeaderMessage::LeaderElectionRequest {
                    request_id,
                    timestamp: _,
                } => {
                    //TODO: usar timestamp
                    if request_id > self.peer_handler.own_id {
                        return;
                    }
                    let guard = thread::spawn(move || {
                        LeaderHandler::send_leader_request(self.peer_handler)
                    });

                    let asker = self
                        .peer_handler
                        .connected_peers
                        .get(&(&request_id))
                        .unwrap();
                    asker.write_message(LeaderMessage::OkMessage {});
                    guard.join()
                }
                LeaderMessage::CoordinatorMessage { connection_id } => {
                    self.actual_leader = connection_id
                }
                LeaderMessage::StillAlive {} => todo!(),
                LeaderMessage::TodoMessage { msg: _ } => todo!(),
                LeaderMessage::OkMessage => {}
            }
        }
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
