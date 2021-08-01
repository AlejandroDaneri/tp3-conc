use std::{io, sync::mpsc::Receiver, thread};

use crate::blockchain::client_event::{ClientMessage, LeaderMessage};

#[derive(Debug)]
pub struct LeaderHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

struct LeaderProcessor {}

impl LeaderHandler {
    pub fn new(leader_receiver: Receiver<LeaderMessage>) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            LeaderHandler::run(leader_receiver).unwrap();
        }));
        LeaderHandler { thread_handle }
    }

    fn run(message_receiver: Receiver<LeaderMessage>) -> io::Result<()> {
        let processor = LeaderProcessor::new();
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
    pub fn new() -> Self {
        LeaderProcessor {}
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
    pub fn leader_processor(&self, receiver: Receiver<LeaderMessage>) {
        while let Ok(message) = receiver.recv() {
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
        }
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
