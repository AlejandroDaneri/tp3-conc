use std::cell::RefCell;
use std::rc::Rc;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread;

use super::client_event::ClientEvent;

#[derive(Debug)]
pub struct Peer {
    pub stream: Rc<RefCell<TcpStream>>,
    id: usize,
    join_handler: thread::JoinHandle<()>,
}

impl Peer {
    pub fn new(id: usize, stream: TcpStream, sender: Sender<ClientEvent>) -> Self {
        let stream_clone = stream.try_clone().unwrap();
        let join_handler = thread::spawn(move || {
            let _id = 0;
            Peer::recv_messages(stream_clone);
            sender.send(ClientEvent::ConnectionError { connection_id: 0 });
        });

        Peer {
            stream: Rc::new(RefCell::new(stream)),
            id,
            join_handler,
        }
    }

    fn recv_messages(_stream: TcpStream) {}
}
