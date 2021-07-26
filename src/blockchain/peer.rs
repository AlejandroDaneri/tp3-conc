use std::io;
use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::client_event::ClientEvent;
use crate::blockchain::client_event::{ClientEventReader, ClientMessage};
use std::io::Write;

#[derive(Debug)]
pub struct Peer {
    id: u32,
    join_handler: thread::JoinHandle<()>,
    stream: Arc<Mutex<TcpStream>>,
}

impl Peer {
    pub fn new(id: u32, stream: TcpStream, sender: Sender<(ClientEvent, Sender<String>)>) -> Self {
        let stream = Arc::new(Mutex::new(stream));
        let stream_clone = stream.clone();

        let join_handler = thread::spawn(move || {
            Peer::recv_messages(id, stream_clone, sender).unwrap();
        });
        Peer {
            id,
            join_handler,
            stream,
        }
    }

    fn recv_messages(
        id: u32,
        stream: Arc<Mutex<TcpStream>>,
        sender: Sender<(ClientEvent, Sender<String>)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (response_sender, response_receiver) = channel();
        let stream_clone = stream.lock().unwrap().try_clone().unwrap();
        let message_reader = ClientEventReader::new(stream_clone, id);
        for message in message_reader {
            sender.send((ClientEvent::Message { message }, response_sender.clone()))?;
            let response = response_receiver.recv()?;
            if let Ok(mut stream) = stream.lock() {
                stream.write(response.as_bytes())?;
            }
        }
        println!("No more events!");
        let message = ClientMessage::ConnectionError { connection_id: id };
        sender.send((ClientEvent::Message { message }, response_sender))?;
        Ok(())
    }

    pub fn write_message(&self, msg: ClientMessage) -> Result<String, io::Error> {
        //mandar msg esperar respuesta , si no respone devolver error
        let response = format!("{}\n", msg);
        if let Ok(mut stream) = self.stream.lock() {
            stream.write(response.as_bytes())?;
        }
        Ok(response)
    }
}
