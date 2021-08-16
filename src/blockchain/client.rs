use std::io;
use std::sync::mpsc::channel;

use crate::blockchain::lock::CentralizedLock;
use crate::blockchain::peer::PeerIdType;
use crate::communication::dispatcher::Dispatcher;
use crate::handler::connection_handler::ConnectionHandler;
use crate::handler::input_handler::InputProcessor;
use crate::handler::leader_handler::LeaderHandler;
use crate::handler::lock_handler::LockProcessor;
use crate::handler::message_handler::MessageHandler;
use crate::handler::peer_handler::PeerHandler;
use std::io::Read;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub struct Client {
    id: PeerIdType,
}

#[allow(clippy::mutex_atomic)]
impl Client {
    pub fn new(id: u32) -> Self {
        Client { id }
    }

    pub fn run<T: 'static + Read + Send>(
        &mut self,
        source: T,
        port_from: u16,
        port_to: u16,
    ) -> io::Result<()> {
        let (leader_handler_sender, leader_handler_receiver) = channel();
        let (peer_handler_sender, peer_handler_receiver) = channel();
        let (message_handler_sender, message_handler_receiver) = channel();
        let (output_sender, output_receiver) = channel();

        let leader_notify = Arc::new((Mutex::new(true), Condvar::new()));
        let leader_handler = LeaderHandler::new(
            leader_handler_receiver,
            peer_handler_sender.clone(),
            output_sender.clone(),
            leader_notify.clone(),
            self.id,
        );

        let lock = CentralizedLock::new();
        let lock_notify = Arc::new((Mutex::new(lock), Condvar::new()));
        let lock_handler = LockProcessor::new(peer_handler_sender.clone(), lock_notify.clone());

        let dispatcher = Dispatcher::new(
            self.id,
            peer_handler_sender,
            message_handler_sender,
            leader_handler_sender.clone(),
            output_sender.clone(),
            lock_handler,
        );

        let connection_handler = ConnectionHandler::new(port_from, port_to, dispatcher.clone());

        let message_handler = MessageHandler::new(
            self.id,
            message_handler_receiver,
            dispatcher.clone(),
            leader_notify,
        );

        let peer_handler = PeerHandler::new(self.id, peer_handler_receiver, dispatcher.clone());

        let input_handler = InputProcessor::new(output_receiver, dispatcher);
        input_handler.run(source);

        drop(connection_handler);
        drop(peer_handler);
        drop(input_handler);
        drop(message_handler);
        drop(leader_handler);

        Ok(())
    }
}
