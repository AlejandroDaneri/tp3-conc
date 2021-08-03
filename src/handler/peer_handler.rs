use crate::blockchain::client_event::{ClientEvent, LeaderMessage};
use crate::blockchain::peer::{Peer, PeerIdType};
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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
        &self,
        own_id: u32,
        sender: Sender<ClientEvent>,
        receiver: Receiver<ClientEvent>,
    ) -> io::Result<()> {
        for event in receiver {
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
                    if let Some(peer) = self.connected_peers.get(&peer_id) {
                        // if let Some(peer) = connected_peers.get(&peer_id) {
                        let sent = peer.write_message(message);
                        if sent.is_err() {
                            println!("Peer {} disconnected!", peer_id);
                            // TODO: leader election leader_handler_sender.send(LeaderMessage::PeerDisconnected...)
                        }
                    }
                }
                ClientEvent::CoordinatorToAll {} => {
                    for (_peer_pid, peer) in self.connected_peers.iter() {
                        peer.write_message_leader(LeaderMessage::CoordinatorMessage {
                            connection_id: own_id,
                        });
                    }
                }

                ClientEvent::LeaderEvent::LeaderElectionRequest {
                    request_id,
                    timestamp,
                } => {
                    self.connected_peers
                        .iter()
                        .filter(|(peer_id, peer)| *peer_id > &own_id)
                        .map(|(peer_id, peer)| {
                            peer.write_message_leader(LeaderMessage::LeaderElectionRequest {
                                request_id,
                                timestamp,
                            })
                        });
                    self.connected_peers
                        .get(&request_id)
                        .unwrap()
                        .write_message_leader(LeaderMessage::OkMessage {});
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
impl PeerHandler {
    pub fn new(
        own_id: PeerIdType,
        response_sender: Sender<ClientEvent>,
        request_receiver: Receiver<ClientEvent>,
        leader_handler_sender: Sender<LeaderMessage>,
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
        _leader_handler_sender: Sender<LeaderMessage>,
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
