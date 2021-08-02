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
    thread_handle: Option<thread::JoinHandle<()>>,
    pub connected_peers: HashMap<u32, Peer>,
    pub own_id: u32,
}

impl PeerHandler {
    pub fn new(
        own_id: PeerIdType,
        response_sender: Sender<ClientEvent>,
        request_receiver: Receiver<ClientEvent>,
        leader_handler_sender: Sender<LeaderMessage>,
    ) -> Self {
        let mut ret = PeerHandler {
            thread_handle: None,
            connected_peers: HashMap::new(),
            own_id,
        };
        ret.thread_handle = Some(thread::spawn(move || {
            ret.run(
                own_id,
                request_receiver,
                response_sender,
                leader_handler_sender,
            )
            .unwrap();
            // let thread_handle = Some(thread::spawn(move || {
            //     PeerHandler::run(
            //         own_id,
            //         request_receiver,
            //         response_sender,
            //         leader_handler_sender,
            //     )
            //     .unwrap();
        }));
        ret
    }

    fn run(
        &mut self,
        own_id: PeerIdType,
        receiver: Receiver<ClientEvent>,
        sender: Sender<ClientEvent>,
        leader_handler_sender: Sender<LeaderMessage>,
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
                _ => unreachable!(),
            }
        }
        Ok(())
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
