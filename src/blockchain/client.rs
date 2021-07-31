use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};

use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::client_event::{ClientEvent, ClientEventReader, ClientMessage};
use crate::blockchain::lock::{CentralizedLock, Lock, LockResult};
use crate::blockchain::peer::{Peer, PeerIdType};
use std::io::{BufRead, BufReader, Error, Write};
use std::str::FromStr;

#[derive(Debug)]
pub struct Client {
    id: u32,
    connected_peers: HashMap<PeerIdType, Peer>,
    lock: CentralizedLock,
    blockchain: Blockchain,
    leader: PeerIdType,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client {
            id,
            connected_peers: HashMap::new(),
            lock: CentralizedLock::new(), // tiene que saber cual es el lider
            blockchain: Blockchain::new(),
            leader: 0,
        }
    }
    // se llama cuando se lo designa coordinador
    fn set_coordinator(&mut self) {
        self.leader = self.id;
    }

    fn update_coordinator(&mut self, id: u32) {
        self.leader = id;
    }

    fn is_leader(&self) -> bool {
        self.id == self.leader
    }

    pub fn run(&mut self, port_from: u16, port_to: u16) -> io::Result<()> {
        let (sender, receiver) = channel();
        let client_sender = sender.clone();

        let listener_handle: JoinHandle<io::Result<()>> = thread::spawn(move || {
            // TODO? listener no bloqueante para poder salir del incoming
            let listener = Client::listen_in_range(port_from, port_to)?;
            let own_port: u16 = listener.local_addr()?.port();
            Client::do_broadcasting(port_from, port_to, &client_sender, own_port)?;
            Client::listen_to_incoming(client_sender, listener)?;
            Ok(())
        });

        let cur_id = 0; // receiver.recv()?

        let input_sender = sender.clone();

        thread::spawn(move || -> io::Result<()> { Client::process_stdin(cur_id, input_sender) });

        self.process_incoming_events(sender, receiver);

        listener_handle.join().unwrap()?;

        Ok(())
    }

    fn process_stdin(cur_id: u32, input_sender: Sender<ClientEvent>) -> Result<(), Error> {
        let source = io::stdin();
        let message_reader = ClientEventReader::new(source, cur_id);
        for message in message_reader {
            println!("Enviando evento {:?}", message);
            input_sender
                .send(ClientEvent::UserInput { message })
                .unwrap();
        }
        println!("Saliendo de la aplicación");
        Ok(())
    }

    fn listen_to_incoming(
        client_sender: Sender<ClientEvent>,
        listener: TcpListener,
    ) -> Result<(), Error> {
        for connection in listener.incoming() {
            let stream = connection?;
            let event = ClientEvent::Connection { stream };
            client_sender.send(event).unwrap();
        }
        Ok(())
    }

    fn do_broadcasting(
        port_from: u16,
        port_to: u16,
        client_sender: &Sender<ClientEvent>,
        own_port: u16,
    ) -> Result<(), Error> {
        for stream in Client::broadcast(own_port, port_from, port_to) {
            let mut stream_clone = stream.try_clone()?;
            let event = ClientEvent::Connection { stream };
            let (response_sender, response_receiver) = channel();
            client_sender.send(event).unwrap();
            let response: String = response_receiver.recv().unwrap();
            stream_clone.write(response.as_bytes())?;
        }
        Ok(())
    }

    fn process_incoming_events(
        &mut self,
        sender: Sender<ClientEvent>,
        receiver: Receiver<ClientEvent>,
    ) -> io::Result<()> {
        //proceso mensajes que me llegan
        while let Ok(event) = receiver.recv() {
            match event {
                ClientEvent::Connection { mut stream } => {
                    let peer_pid = self.exchange_pids(&mut stream)?;
                    let peer = Peer::new(peer_pid, stream, sender.clone());
                    self.connected_peers.insert(peer_pid, peer);
                }
                ClientEvent::PeerMessage { message, peer_id } => {
                    if let Some(response) = self.process_message(message, peer_id) {
                        if let Some(peer) = self.connected_peers.get(&peer_id) {
                            let sent = peer.write_message(response);
                            if sent.is_err() {
                                println!("Peer {} disconnected!", peer_id);
                                // TODO: leader election
                            }
                        }
                    }
                }
                ClientEvent::PeerDisconnected { peer_id } => {
                    self.connected_peers.remove(&peer_id);
                    println!("Peer {} removed", peer_id);
                    // TODO: leader election
                }
                ClientEvent::UserInput { message } => {
                    // TODO ¿Poner un process_input más especializado? ¿Usar otro enum de mensajes?
                    self.process_message(message, self.id);
                }
            }
        }
        Ok(())
    }

    fn process_message(&mut self, message: ClientMessage, peer_id: u32) -> Option<ClientMessage> {
        println!("PROCESS: {:?}", message.serialize());
        match message {
            ClientMessage::ReadBlockchainRequest {} => {
                if !self.lock.is_owned_by(peer_id) {
                    return Some(ClientMessage::TodoMessage {
                        msg: "rb lock not acquired previosly".to_owned(),
                    });
                }
                if self.is_leader() {
                    Some(ClientMessage::ReadBlockchainResponse {
                        blockchain: self.blockchain.clone(),
                    })
                } else {
                    Some(ClientMessage::TodoMessage {
                        msg: "rb with no leader".to_owned(),
                    })
                }
            }
            ClientMessage::ReadBlockchainResponse { blockchain } => {
                println!("Blockchain: {}", blockchain);
                None
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if self.is_leader() {
                    {
                        let valid = self.blockchain.validate(transaction.clone()); //esto deberia ser la transaccion que recibe cuando devuelve el lock
                        self.blockchain.add_transaction(transaction);
                    }
                }
                Some(ClientMessage::TodoMessage { msg: format!("wb") })
            }

            ClientMessage::LockRequest {
                read_only,
                request_id,
            } => {
                // si me llega esto deberia ser lider
                // soy lider?
                if self.lock.acquire(request_id) == LockResult::Acquired {
                    Some(ClientMessage::TodoMessage { msg: format!("lock acquired") })
                } else {
                    Some(ClientMessage::TodoMessage { msg: format!("lock failed") })
                }
            }

            ClientMessage::LeaderElectionRequest {
                request_id,
                timestamp: _,
            } => {
                //TODO: usar timestamp
                if request_id > self.id {
                    return Some(ClientMessage::TodoMessage {
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
                Some(ClientMessage::TodoMessage {
                    msg: "Bully OK".to_owned(),
                })
            }
            ClientMessage::OkMessage {} => None,

            ClientMessage::CoordinatorMessage { connection_id: id } => {
                self.update_coordinator(id);
                if self.leader != self.id {
                    println!("New leader: {}", id);
                }
                Some(ClientMessage::TodoMessage {
                    msg: format!("CoordinatorUpdate {}", id),
                })
            }
            ClientMessage::StillAlive {} => None,
            ClientMessage::TodoMessage { msg: _msg } => None,
        }
    }

    fn broadcast(own_port: u16, port_from: u16, port_to: u16) -> Vec<TcpStream> {
        let host = "localhost";
        (port_from..port_to)
            .into_iter()
            .filter(|port| *port != own_port)
            .map(|port| ((host), port))
            .map(TcpStream::connect)
            .flatten()
            .collect()
    }

    fn listen_in_range(port_from: u16, port_to: u16) -> io::Result<TcpListener> {
        let mask = [127, 0, 0, 1];
        let mut addrs = vec![];
        for port in port_from..port_to {
            addrs.push(SocketAddr::from((mask, port)))
        }

        match TcpListener::bind(&addrs[..]) {
            Ok(listener) => Ok(listener),
            Err(_err) => Err(io::Error::new(io::ErrorKind::Other, "Pool not available")),
        }
    }

    fn send_result(&mut self, _id: u32, _result: u32) {}
    fn send_modifications(&mut self, _id: u32, _result: u32) {}

    fn notify_minions(&self, _id: u32) {
        println!("----notify----");
        for (peer_pid, peer) in self.connected_peers.iter() {
            peer.write_message(ClientMessage::CoordinatorMessage {
                connection_id: self.id,
            });
        }
    }
    fn send_leader_request(&mut self, id: u32) {
        let mut higher_alive = false;
        println!("MANDE LIDER");
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
    }

    fn send_request_to_leader(&self, message: ClientMessage) -> io::Result<()> {
        if let Some(leader_peer) = self.connected_peers.get(&self.leader) {
            leader_peer.write_message(message)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Request sent to none leader",
            ))
        }
    }

    fn get_leader_id(&mut self, _id: u32, _result: u32) -> u32 {
        0
    }

    fn exchange_pids(&self, stream: &mut TcpStream) -> io::Result<u32> {
        let pid_msg = format!("{}\n", self.id);
        stream.write(pid_msg.as_bytes())?;
        let mut buf_reader = BufReader::new(stream);
        let mut client_pid = String::new();
        buf_reader.read_line(&mut client_pid)?;
        client_pid.pop();
        u32::from_str(&client_pid)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, "bad client pid"))
    }
}
