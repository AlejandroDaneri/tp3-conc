use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

use super::blockchain::Blockchain;

use super::client_event::ClientEvent;
use super::lock::CentralizedLock;
use super::peer::Peer;

#[derive(Debug)]
pub struct Client {
    id: u32,
    connected_peers: Vec<Peer>,
    lock: Option<CentralizedLock>,
    blockchain: Blockchain,
    leader: u32,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client {
            id,
            connected_peers: vec![],
            lock: None,
            blockchain: Blockchain::new(),
            leader: 0,
        }
    }

    pub fn run(&mut self, port_from: u16, port_to: u16) -> io::Result<()> {
        let (sender, receiver) = channel();
        let client_sender = sender.clone();

        let listener_handle: JoinHandle<io::Result<()>> = thread::spawn(move || {
            // TODO? listener no bloqueante para poder salir del incoming
            let listener = Client::listen_in_range(port_from, port_to)?;
            let own_port: u16 = listener.local_addr()?.port();
            for stream in Client::broadcast(own_port, port_from, port_to).into_iter() {
                let event = ClientEvent::Connection { stream };
                client_sender.send(event).unwrap();
            }
            for connection in listener.incoming() {
                let event = ClientEvent::Connection {
                    stream: connection?,
                };
                client_sender.send(event).unwrap();
            }
            Ok(())
        });

        self.process_messages(sender, receiver);

        listener_handle.join().unwrap()?;

        Ok(())
    }

    fn process_messages(&mut self, sender: Sender<ClientEvent>, receiver: Receiver<ClientEvent>) {
        let mut cur_id = 0;
        //proceso mensajes que me llegan
        while let Ok(message) = receiver.recv() {
            match message {
                ClientEvent::Connection { stream } => {
                    println!("Connection: {:?}", stream);
                    let peer = Peer::new(cur_id, stream, sender.clone());
                    cur_id += 1;
                    self.connected_peers.push(peer);
                }
                ClientEvent::ReadBlockchainRequest {} => {
                    // si me llega esto deberia ser lider
                    // soy lider?
                    // no, ToDo (error?, redirigirlo al lider?)
                    // si ->
                    let coord_stream_cell: Rc<RefCell<TcpStream>> = self.get_stream(self.id);
                    let mut coord_stream = coord_stream_cell.borrow_mut();
                    self.lock.acquire(true, &mut coord_stream); // <- esto deberia ser lock local, porque soy lider
                    self.blockchain.refresh(); // aca seria devolver la blockchain en vez de esto??
                                               // println!(blockchain);
                    self.lock.release(&mut coord_stream); // <- esto deberia ser lock local, porque soy lider. Extrapolar con los demas msjs
                }
                ClientEvent::WriteBlockchainRequest { transaction } => {
                    // si me llega esto deberia ser lider
                    // soy lider?
                    // no, ToDo (error?, redirigirlo al lider?)
                    // si ->
                    let coord_stream_cell: Rc<RefCell<TcpStream>> = self.get_stream(self.id);
                    let mut coord_stream = coord_stream_cell.borrow_mut();
                    self.lock.acquire(false, &mut coord_stream);
                    self.blockchain.refresh();
                    let _modifications = self.blockchain.add_transaction(transaction);
                    self.send_modifications(0, 0); //TODO: ver si va
                    self.lock.release(&mut coord_stream);
                }
                ClientEvent::LockRequest {
                    read_only,
                    request_id,
                } => {
                    // si me llega esto deberia ser lider
                    // soy lider?
                    // no, ToDo (error?)
                    // si ->
                    let coord_stream_cell: Rc<RefCell<TcpStream>> = self.get_stream(request_id);
                    let mut coord_stream = coord_stream_cell.borrow_mut();
                    let _result = self.lock.acquire(read_only, &mut coord_stream);
                    self.send_result(request_id, 0);
                }
                ClientEvent::LeaderElectionRequest {
                    request_id: _,
                    timestamp: _,
                } => {
                    self.send_leader_request(0, 0);
                    self.leader = self.get_leader_id(0, 0);
                    if self.leader == self.id {
                        //si el lider soy yo
                        self.notify_minions(0, 0);
                    }
                }
                ClientEvent::ConnectionError { connection_id: _ } => {
                    //self.connected_peers.filter_by_id(id);
                }
                ClientEvent::OkMessage {} => {
                    // hay alguien mayor que esta vivo, o sea no voy a ser lider
                }
                ClientEvent::CoordinatorMessage {} => {
                    //no me toca ser lider esta vez :(
                    //aca deberia setear todas las comunicaciones al nuevo coordinador
                }
            }
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

    fn get_stream(&mut self, _id: u32) -> Rc<RefCell<TcpStream>> {
        let ref peer = self.connected_peers[0];
        peer.stream.clone()
    }

    fn send_result(&mut self, _id: u32, _result: u32) {}
    fn send_modifications(&mut self, _id: u32, _result: u32) {}

    fn notify_minions(&mut self, _id: u32, _result: u32) {
        //enviar a todos mi stream y/o canal para que me pidan lock y esas cosas
    }
    fn send_leader_request(&mut self, _id: u32, _result: u32) {}
    fn get_leader_id(&mut self, _id: u32, _result: u32) -> u32 {
        return 0;
    }
}
