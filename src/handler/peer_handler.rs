use crate::blockchain::peer::{Peer, PeerIdType};
use crate::communication::client_event::{ClientEvent, ClientMessage, LeaderMessage, Message};
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
}

pub struct PeerProcessor {
    connected_peers: HashMap<u32, Peer>,
    own_id: PeerIdType,
    sender: Sender<ClientEvent>,
    receiver: Receiver<ClientEvent>,
    leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
    output_sender: Sender<ClientMessage>,
}

impl PeerProcessor {
    pub fn new(
        connected_peers: HashMap<u32, Peer>,
        own_id: PeerIdType,
        sender: Sender<ClientEvent>,
        receiver: Receiver<ClientEvent>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
    ) -> Self {
        Self {
            connected_peers,
            own_id,
            sender,
            receiver,
            leader_handler_sender,
            output_sender,
        }
    }
    pub fn process(&mut self) -> io::Result<()> {
        for event in self.receiver.iter() {
            println!("PH: Processing event: {:?}", event);
            match event {
                ClientEvent::Connection { mut stream } => {
                    let peer_pid = PeerHandler::exchange_pids(self.own_id, &mut stream)?;
                    self.leader_handler_sender
                        .send((LeaderMessage::SendLeaderId {}, peer_pid))
                        .ok();
                    let peer = Peer::new(peer_pid, stream, self.sender.clone());
                    self.connected_peers.insert(peer_pid, peer);
                }
                ClientEvent::PeerDisconnected { peer_id } => {
                    self.connected_peers.remove(&peer_id);
                    println!("Peer {} removed", peer_id);
                    let message = LeaderMessage::PeerDisconnected;
                    self.leader_handler_sender.send((message, peer_id)).ok();
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    self.handle_peer_message(message, peer_id);
                }
                ClientEvent::UserInput { .. } => unreachable!(),
            }
        }
        Ok(())
    }

    fn handle_peer_message(&self, message: Message, peer_id: PeerIdType) {
        match message {
            Message::Common(ClientMessage::BroadcastBlockchain { blockchain }) => {
                for (_, peer) in self.connected_peers.iter() {
                    peer.send_message(Message::Common(ClientMessage::ReadBlockchainResponse {
                        blockchain: blockchain.clone(),
                    }))
                    .ok();
                }
            }
            Message::Common(inner) => match self.connected_peers.get(&peer_id) {
                Some(peer) => {
                    let sent = peer.send_message(Message::Common(inner));
                    if sent.is_err() {
                        println!("Peer {} disconnected!", peer_id);
                        let message = LeaderMessage::PeerDisconnected;
                        self.leader_handler_sender.send((message, peer_id)).ok();
                    }
                }
                None => {
                    println!("[{}] Peer not found: {}", self.own_id, peer_id);
                    if peer_id != self.own_id {
                        let message = LeaderMessage::PeerDisconnected;
                        self.leader_handler_sender.send((message, peer_id)).ok();
                    } else {
                        self.output_sender.send(inner).ok();
                    }
                }
            },
            Message::Leader(message) => match message {
                LeaderMessage::LeaderElectionRequest { .. } => {
                    self.connected_peers
                        .iter()
                        .filter(|(&peer_id, _)| peer_id > self.own_id)
                        .for_each(|(peer_id, peer)| {
                            println!("Pidiendo ser lider a {}", peer_id);
                            let msg = Message::Leader(LeaderMessage::LeaderElectionRequest {
                                timestamp: SystemTime::now(),
                            });
                            peer.send_message(msg).ok();
                        });
                }
                LeaderMessage::OkMessage {} => {
                    if let Some(peer) = self.connected_peers.get(&peer_id) {
                        let sent = peer.send_message(Message::Leader(message));
                        if sent.is_err() {
                            println!("Peer {} disconnected!", peer_id);
                        }
                    }
                }
                LeaderMessage::VictoryMessage {} => {
                    for (peer_id, peer) in self.connected_peers.iter() {
                        println!("Send victory to {}!", peer_id);
                        peer.send_message(Message::Leader(LeaderMessage::VictoryMessage {}))
                            .ok();
                    }
                }
                _ => unreachable!(),
            },
            Message::Lock(inner) => {
                if let Some(peer) = self.connected_peers.get(&peer_id) {
                    let sent = peer.send_message(Message::Lock(inner));
                    if sent.is_err() {
                        println!("Peer {} disconnected!", peer_id);
                        let message = LeaderMessage::PeerDisconnected;
                        self.leader_handler_sender.send((message, peer_id)).ok();
                    }
                }
            }
        }
    }
}
impl PeerHandler {
    pub fn new(
        own_id: PeerIdType,
        request_receiver: Receiver<ClientEvent>,
        response_sender: Sender<ClientEvent>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
    ) -> Self {
        let connected_peers = HashMap::new();
        let thread_handle = thread::spawn(move || {
            PeerHandler::run(
                own_id,
                request_receiver,
                response_sender,
                leader_handler_sender,
                output_sender,
                connected_peers,
            )
        });
        PeerHandler {
            thread_handle: Some(thread_handle),
        }
    }

    fn run(
        own_id: PeerIdType,
        receiver: Receiver<ClientEvent>,
        sender: Sender<ClientEvent>,
        leader_handler_sender: Sender<(LeaderMessage, PeerIdType)>,
        output_sender: Sender<ClientMessage>,
        connected_peers: HashMap<u32, Peer>,
    ) -> io::Result<()> {
        let mut processor = PeerProcessor::new(
            connected_peers,
            own_id,
            sender,
            receiver,
            leader_handler_sender,
            output_sender,
        );
        processor.process()
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
