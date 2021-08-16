use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};

use crate::communication::client_event::ClientEvent;
use crate::communication::dispatcher::Dispatcher;
use std::thread;

#[derive(Debug)]
pub struct ConnectionHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ConnectionHandler {
    pub fn new(port_from: u16, port_to: u16, dispatcher: Dispatcher) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            ConnectionHandler::run(port_from, port_to, dispatcher).unwrap();
        }));
        ConnectionHandler { thread_handle }
    }

    fn run(port_from: u16, port_to: u16, dispatcher: Dispatcher) -> io::Result<()> {
        let listener = ConnectionHandler::listen_in_range(port_from, port_to)?;
        let own_port: u16 = listener.local_addr()?.port();
        ConnectionHandler::do_broadcasting(port_from, port_to, own_port, &dispatcher)?;
        ConnectionHandler::listen_to_incoming(listener, &dispatcher)?;
        Ok(())
    }

    fn do_broadcasting(
        port_from: u16,
        port_to: u16,
        own_port: u16,
        dispatcher: &Dispatcher,
    ) -> io::Result<()> {
        for stream in ConnectionHandler::broadcast(own_port, port_from, port_to) {
            let event = ClientEvent::Connection {
                stream,
                incoming: false,
            };
            dispatcher.dispatch(event).unwrap();
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

    fn listen_to_incoming(listener: TcpListener, dispatcher: &Dispatcher) -> io::Result<()> {
        for connection in listener.incoming() {
            let stream = connection?;
            let event = ClientEvent::Connection {
                stream,
                incoming: true,
            };
            dispatcher.dispatch(event).unwrap();
        }
        Ok(())
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
