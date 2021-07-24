use std::cell::RefCell;
use std::net::TcpStream;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use super::client_event::ClientEvent;
use std::io::{BufReader, BufRead, Write};
use crate::blockchain::client_event::ClientEventReader;

#[derive(Debug)]
pub struct Peer {
    id: u32,
    join_handler: thread::JoinHandle<()>,
}

impl Peer {
    pub fn new(id: u32, mut stream: TcpStream, sender: Sender<(ClientEvent, Sender<String>)>) -> Self {
        let join_handler = thread::spawn(move || {
             Peer::recv_messages(id, stream, sender);
        });

        Peer {
            id,
            join_handler,
        }
    }

    fn recv_messages(id: u32, mut stream: TcpStream, sender: Sender<(ClientEvent, Sender<String>)>) {
        let (response_sender, response_receiver) = channel();
        let stream_clone = stream.try_clone().unwrap();
        let event_reader = ClientEventReader::new(stream_clone, id);
        for event in event_reader {
            sender.send((event, response_sender.clone()));
            let response = response_receiver.recv().expect("sender closed unexpectedly");
            stream.write(response.as_bytes());
        };
        sender
            .send((ClientEvent::ConnectionError { connection_id: id }, response_sender))
            .unwrap();
    }
}
