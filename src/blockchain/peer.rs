use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use super::client_event::ClientEvent;
use crate::blockchain::client_event::{ClientEventReader, ClientMessage};
use std::io::Write;

#[derive(Debug)]
pub struct Peer {
    id: u32,
    join_handler: thread::JoinHandle<()>,
}

impl Peer {
    pub fn new(id: u32, stream: TcpStream, sender: Sender<(ClientEvent, Sender<String>)>) -> Self {
        let join_handler = thread::spawn(move || {
            Peer::recv_messages(id, stream, sender);
        });

        Peer { id, join_handler }
    }

    fn recv_messages(
        id: u32,
        mut stream: TcpStream,
        sender: Sender<(ClientEvent, Sender<String>)>,
    ) {
        let (response_sender, response_receiver) = channel();
        let stream_clone = stream.try_clone().unwrap();
        let message_reader = ClientEventReader::new(stream_clone, id);
        for message in message_reader {
            sender.send((ClientEvent::Message { message }, response_sender.clone()));
            let response = response_receiver
                .recv()
                .expect("sender closed unexpectedly");
            stream.write(response.as_bytes());
        }
        println!("No more events!");
        let message = ClientMessage::ConnectionError { connection_id: id };
        sender
            .send((ClientEvent::Message { message }, response_sender))
            .unwrap();
    }
}
