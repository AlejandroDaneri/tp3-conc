use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Sender;

use crate::communication::client_event::ClientEvent;
use std::thread;

#[derive(Debug)]
pub struct ConnectionHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ConnectionHandler {
    pub fn new(client_sender: Sender<ClientEvent>, port_from: u16, port_to: u16) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            ConnectionHandler::run(client_sender, port_from, port_to).unwrap();
        }));
        ConnectionHandler { thread_handle }
    }

    fn run(client_sender: Sender<ClientEvent>, port_from: u16, port_to: u16) -> io::Result<()> {
        let listener = ConnectionHandler::listen_in_range(port_from, port_to)?;
        let own_port: u16 = listener.local_addr()?.port();
        ConnectionHandler::do_broadcasting(port_from, port_to, &client_sender, own_port)?;
        ConnectionHandler::listen_to_incoming(client_sender, listener)?;
        Ok(())
    }

    fn do_broadcasting(
        port_from: u16,
        port_to: u16,
        client_sender: &Sender<ClientEvent>,
        own_port: u16,
    ) -> io::Result<()> {
        for stream in ConnectionHandler::broadcast(own_port, port_from, port_to) {
            let event = ClientEvent::Connection {
                stream,
                incoming: false,
            };
            client_sender.send(event).unwrap();
        }
        Ok(())
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

    fn listen_to_incoming(
        client_sender: Sender<ClientEvent>,
        listener: TcpListener,
    ) -> io::Result<()> {
        for connection in listener.incoming() {
            let stream = connection?;
            let event = ClientEvent::Connection {
                stream,
                incoming: true,
            };
            client_sender.send(event).unwrap();
        }
        Ok(())
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
