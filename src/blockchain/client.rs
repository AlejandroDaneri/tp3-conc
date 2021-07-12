use std::io;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

enum ClientEvent {
    Connection { stream: TcpStream },
}

#[derive(Debug)]
pub struct Client {
    id: u32,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Client { id }
    }

    pub fn run(&self, port_from: u16, port_to: u16) -> io::Result<()> {
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

    fn process_messages(&self, sender: Sender<ClientEvent>, receiver: Receiver<ClientEvent>) {
        while let Ok(message) = receiver.recv() {
            match message {
                ClientEvent::Connection { stream } => {
                    println!("Connection: {:?}", stream);
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
}
