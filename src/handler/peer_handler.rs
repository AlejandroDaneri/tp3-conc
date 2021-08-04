use crate::blockchain::client_event::{ClientEvent, LeaderMessage};
use crate::blockchain::peer::{Peer, PeerIdType};
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::SystemTime;

#[derive(Debug)]
pub struct PeerHandler {
    thread_handle: Option<thread::JoinHandle<io::Result<()>>>,
    pub own_id: u32,
}
pub struct PeerProcessor {
    connected_peers: HashMap<u32, Peer>,
}

impl PeerProcessor {
    pub fn new(connected_peers: HashMap<u32, Peer>) -> Self {
        Self { connected_peers }
    }
    pub fn process(
        &mut self,
        own_id: u32,
        sender: Sender<ClientEvent>,
        receiver: Receiver<ClientEvent>,
    ) -> io::Result<()> {
        for event in receiver {
            println!("PH: Processing event: {:?}", event);
            match event {
                ClientEvent::Connection { mut stream } => {
                    let peer_pid = PeerHandler::exchange_pids(own_id, &mut stream)?;
                    let peer = Peer::new(peer_pid, stream, sender.clone());
                    self.connected_peers.insert(peer_pid, peer);
                }
                ClientEvent::PeerDisconnected { peer_id } => {
                    self.connected_peers.remove(&peer_id);
                    println!("Peer {} removed", peer_id);
                    // TODO: leader election leader_handler_sender.send(LeaderMessage::PeerDisconnected...)
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    println!("sending message to {}: {:?}", peer_id, message);
                    match self.connected_peers.get(&peer_id) {
                        Some(peer) => {
                            let sent = peer.write_message(message);
                            if sent.is_err() {
                                println!("Peer {} disconnected!", peer_id);
                                // TODO: leader election leader_handler_sender.send(LeaderMessage::PeerDisconnected...)
                            }
                        }
                        None => {
                            println!("Peer {} disconnected!", peer_id);
                            // lo mismo que handleo de LeaderMessage::LeaderElectionRequest
                            self.connected_peers
                                .iter()
                                .filter(|(&peer_idd, _)| peer_idd > own_id)
                                .for_each(|(peer_idd, peer)| {
                                    println!("Pidiendo ser lider a {}", peer_idd);
                                    peer.write_message_leader(
                                        LeaderMessage::LeaderElectionRequest {
                                            timestamp: SystemTime::now(),
                                        },
                                    );
                                });
                            // TODO: el mensaje que fallo no se reenvia
                        }
                    }
                }
                ClientEvent::LeaderEvent { message, peer_id } => match message {
                    LeaderMessage::LeaderElectionRequest { .. } => {
                        self.connected_peers
                            .iter()
                            .filter(|(&peer_id, _)| peer_id > own_id)
                            .for_each(|(peer_id, peer)| {
                                println!("Pidiendo ser lider a {}", peer_id);
                                peer.write_message_leader(LeaderMessage::LeaderElectionRequest {
                                    timestamp: SystemTime::now(),
                                });
                            });
                    }
                    LeaderMessage::OkMessage {} => {
                        if let Some(peer) = self.connected_peers.get(&peer_id) {
                            let sent = peer.write_message_leader(message);
                            if sent.is_err() {
                                println!("Peer {} disconnected!", peer_id);
                            }
                        }
                    }
                    LeaderMessage::VictoryMessage {} => {
                        for (peer_id, peer) in self.connected_peers.iter() {
                            println!("Send victory to {}!", peer_id);
                            peer.write_message_leader(LeaderMessage::VictoryMessage {});
                        }
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
impl PeerHandler {
    pub fn new(
        own_id: PeerIdType,
        request_receiver: Receiver<ClientEvent>,
        response_sender: Sender<ClientEvent>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
    ) -> Self {
        let hash = HashMap::new();
        let thread_handle = thread::spawn(move || {
            PeerHandler::run(
                own_id,
                request_receiver,
                response_sender,
                leader_handler_sender,
                hash,
            )
        });
        PeerHandler {
            thread_handle: Some(thread_handle),
            own_id,
        }
    }

    fn run(
        own_id: PeerIdType,
        receiver: Receiver<ClientEvent>,
        sender: Sender<ClientEvent>,
        _leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        hash: HashMap<u32, Peer>,
    ) -> io::Result<()> {
        let mut processor = PeerProcessor::new(hash);
        processor.process(own_id, sender, receiver)
    }

    fn exchange_pids(own_id: PeerIdType, stream: &mut TcpStream) -> io::Result<u32> {
        let pid_msg = format!("{}\n", own_id);
        stream.write_all(pid_msg.as_bytes())?;
        let mut buf_reader = BufReader::new(stream);
        let mut client_pid = String::new();
        buf_reader.read_line(&mut client_pid)?;
        client_pid.pop();
        u32::from_str(&client_pid)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "bad client pid"))
    }
}

impl Drop for PeerHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
