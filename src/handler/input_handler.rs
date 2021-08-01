use std::io;
use std::sync::mpsc::Sender;
use std::thread;

use crate::blockchain::client_event::{ClientEvent, ClientEventReader};

#[derive(Debug)]
pub struct InputHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl InputHandler {
    pub fn new(input_sender: Sender<ClientEvent>) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            InputHandler::run(input_sender).unwrap();
        }));
        InputHandler { thread_handle }
    }

    fn run(input_sender: Sender<ClientEvent>) -> io::Result<()> {
        let source = io::stdin();
        let message_reader = ClientEventReader::new(source);
        for message in message_reader {
            println!("Enviando evento {:?}", message);
            input_sender
                .send(ClientEvent::UserInput { message })
                .unwrap();
        }
        println!("Saliendo de la aplicación");
        Ok(())
    }
}

impl Drop for InputHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}
