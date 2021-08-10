use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::communication::client_event::ClientEvent;
use crate::communication::client_event::Message;
use crate::communication::serialization::LineReader;

pub type PeerIdType = u32;

#[derive(Debug)]
pub struct Peer {
    id: PeerIdType,
    recv_thread: Option<thread::JoinHandle<()>>,
    send_thread: Option<thread::JoinHandle<()>>,
    sender: Option<Sender<ClientEvent>>,
}

impl Peer {
    pub fn new(id: u32, stream: TcpStream, sender: Sender<ClientEvent>) -> Self {
        let stream_clone = stream.try_clone().unwrap();
        let (local_sender, receiver) = channel();

        let recv_thread = Some(thread::spawn(move || {
            Peer::recv_messages(id, stream, sender).unwrap();
        }));

        let send_thread = Some(thread::spawn(move || {
            Peer::send_messages(stream_clone, receiver).unwrap();
        }));
        Peer {
            id,
            recv_thread,
            send_thread,
            sender: Some(local_sender),
        }
    }

    fn recv_messages(
        peer_id: u32,
        stream: TcpStream,
        sender: Sender<ClientEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message_reader = LineReader::new(stream);
        for message in message_reader {
            let event = ClientEvent::PeerMessage { message, peer_id };
            sender.send(event)?;
        }
        println!("No more events from {}", peer_id);
        sender.send(ClientEvent::PeerDisconnected { peer_id })?;
        Ok(())
    }

    fn send_messages(
        mut stream: TcpStream,
        receiver: Receiver<ClientEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for event in receiver {
            let buf = event.serialize();
            stream.write_all(buf.as_bytes())?;
        }
        Ok(())
    }

    pub fn send_message(&self, msg: Message) -> io::Result<()> {
        match &self.sender {
            Some(sender) => sender
                .send(ClientEvent::UserInput { message: msg })
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "Error while sending message to peer")
                }),
            None => unreachable!(),
        }
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.sender.take();
        let _ = self.recv_thread.take().unwrap().join();
        let _ = self.send_thread.take().unwrap().join();
    }
}
