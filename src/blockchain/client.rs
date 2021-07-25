use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};

use std::thread;
use std::thread::JoinHandle;

use super::blockchain::Blockchain;
use super::client_event::{ClientEvent, ClientEventReader, ClientMessage};
use super::lock::CentralizedLock;
use super::peer::Peer;
use std::io::{BufRead, BufReader, Error, Write};
use std::str::FromStr;

#[derive(Debug)]
pub struct Client {
    id: u32,
    connected_peers: HashMap<u16, Peer>,
    centralized_lock: Option<CentralizedLock>,
    blockchain: Blockchain,
    leader: u32,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client {
            id,
            connected_peers: HashMap::new(),
            centralized_lock: None, // tiene que saber cual es el lider
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
            Client::do_broadcasting(port_from, port_to, &client_sender, own_port);
            Client::listen_to_incoming(client_sender, listener);
            Ok(())
        });

        let cur_id = 0; // receiver.recv()?

        let input_sender = sender.clone();

        thread::spawn(move || -> io::Result<()> { Client::process_stdin(cur_id, input_sender) });

        self.process_incoming_events(sender, receiver);

        listener_handle.join().unwrap()?;

        Ok(())
    }

    fn process_stdin(
        cur_id: u32,
        input_sender: Sender<(ClientEvent, Sender<String>)>,
    ) -> Result<(), Error> {
        let source = io::stdin();
        let message_reader = ClientEventReader::new(source, cur_id);
        let (response_sender, response_receiver) = channel();
        for message in message_reader {
            println!("Enviando evento {:?}", message);
            input_sender.send((ClientEvent::Message { message }, response_sender.clone()));
            let response = response_receiver
                .recv()
                .expect("sender closed unexpectedly");
            println!("{}", response);
        }
        println!("Saliendo de la aplicación");
        Ok(())
    }

    fn listen_to_incoming(
        client_sender: Sender<(ClientEvent, Sender<String>)>,
        listener: TcpListener,
    ) -> Result<(), Error> {
        for connection in listener.incoming() {
            let stream = connection?;
            let mut stream_clone = stream.try_clone().unwrap();
            let event = ClientEvent::Connection { stream };
            let (response_sender, response_receiver) = channel();
            client_sender
                .send((event, response_sender.clone()))
                .unwrap();
            let response = response_receiver.recv().unwrap();
            stream_clone.write(response.as_bytes())?;
        }
        Ok(())
    }

    fn do_broadcasting(
        port_from: u16,
        port_to: u16,
        client_sender: &Sender<(ClientEvent, Sender<String>)>,
        own_port: u16,
    ) -> Result<(), Error> {
        for stream in Client::broadcast(own_port, port_from, port_to).into_iter() {
            let mut stream_clone = stream.try_clone()?;
            let event = ClientEvent::Connection { stream };
            let (response_sender, response_receiver) = channel();
            client_sender
                .send((event, response_sender.clone()))
                .unwrap();
            let response: String = response_receiver.recv().unwrap();
            stream_clone.write(response.as_bytes())?;
        }
        Ok(())
    }

    fn process_incoming_events(
        &mut self,
        sender: Sender<(ClientEvent, Sender<String>)>,
        receiver: Receiver<(ClientEvent, Sender<String>)>,
    ) {
        //proceso mensajes que me llegan
        while let Ok((event, response_sender)) = receiver.recv() {
            if let Some(response) = self.process_event(event, &sender) {
                response_sender.send(response);
            }
        }
    }

    fn process_event(
        &mut self,
        event: ClientEvent,
        sender: &Sender<(ClientEvent, Sender<String>)>,
    ) -> Option<String> {
        match event {
            ClientEvent::Connection { mut stream } => {
                let peer_pid = self.exchange_pids(&mut stream).ok()?;
                let peer = Peer::new(peer_pid, stream, sender.clone());
                self.connected_peers.insert(peer_pid as u16, peer);
                let message = format!("Coordinator {}", self.leader);
                Some(message)
            }
            ClientEvent::Message { message } => self.process_message(message),
        }
    }

    fn process_message(&mut self, message: ClientMessage) -> Option<String> {
        match message {
            ClientMessage::ReadBlockchainRequest {} => {
                let response;
                if self.is_leader() {
                    {
                        //necesita ser lider para devolver??

                        // fijarse si esta lockeado, si va todo bien, entonces ->
                        //let stream_to_peer: Rc<RefCell<TcpStream>> = self.get_stream(port);
                        //let mut stream = stream_to_peer.borrow_mut();
                        response = self.blockchain.as_str();
                        //stream.write(body.as_bytes());
                        // println!(blockchain);
                        // self.lock.unlock();
                    }
                } else {
                    response = "TODO: No leader!".to_owned();
                }
                Some(response)
            }
            ClientMessage::WriteBlockchainRequest { transaction } => {
                if self.is_leader() {
                    {
                        //if not locked
                        /*let stream_to_peer: Rc<RefCell<TcpStream>> = self.get_stream(port);
                        let mut stream = &*stream_to_peer.borrow_mut();
                        stream.write("You have the lock".as_bytes());
                        stream.set_read_timeout(Some(Duration::new(1000, 0)));
                        stream.read(&mut [0; 128]); //espera que le mande la transacion
                        */
                        let valid = self.blockchain.validate(transaction.clone()); //esto deberia ser la transaccion que recibe cuando devuelve el lock
                        self.blockchain.add_transaction(transaction);
                        let body = self.blockchain.as_str();
                    }
                }
                Some("TODO: WriteBlockchainRequest response".to_owned())
            }

            // ClientEvent::LockRequest {
            //     read_only,
            //     request_id,
            // } => {
            //     // si me llega esto deberia ser lider
            //     // soy lider?
            //     // no, ToDo (error?)
            //     // si ->
            //     let coord_stream_cell: Rc<RefCell<TcpStream>> = self.get_stream(request_id);
            //     let mut coord_stream = coord_stream_cell.borrow_mut();
            //     let _result = self.lock.acquire(read_only, &mut coord_stream);
            //     self.send_result(request_id, 0);
            // }
            ClientMessage::LeaderElectionRequest {
                request_id: _,
                timestamp: _,
            } => {
                self.send_leader_request(0, 0);
                self.leader = self.get_leader_id(0, 0);
                if self.leader == self.id {
                    //si el lider soy yo
                    self.notify_minions(0, 0);
                }
                Some("TODO: LeaderElectionRequest response".to_owned())
            }
            ClientMessage::ConnectionError { connection_id } => {
                let id = connection_id as u16;
                self.connected_peers.remove(&id);
                None
            }
            ClientMessage::OkMessage {} => None,
            ClientMessage::CoordinatorMessage { connection_id: id } => {
                self.update_coordinator(id);
                Some("TODO: CoordinatorMessageResponse?".to_owned())
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

    fn send_result(&mut self, _id: u32, _result: u32) {}
    fn send_modifications(&mut self, _id: u32, _result: u32) {}

    fn notify_minions(&mut self, _id: u32, _result: u32) {
        //enviar a todos mi stream y/o canal para que me pidan lock y esas cosas
    }
    fn send_leader_request(&mut self, _id: u32, _result: u32) {}
    fn get_leader_id(&mut self, _id: u32, _result: u32) -> u32 {
        0
    }

    fn exchange_pids(&self, stream: &mut TcpStream) -> io::Result<u32> {
        let pid_msg = format!("{}\n", self.id);
        stream.write(pid_msg.as_bytes())?;
        let mut bufReader = BufReader::new(stream);
        let mut client_pid = String::new();
        bufReader.read_line(&mut client_pid)?;
        client_pid.pop();
        u32::from_str(&client_pid)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, "bad client pid"))
    }
}
