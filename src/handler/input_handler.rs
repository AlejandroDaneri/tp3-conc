use std::io;
use std::sync::mpsc::Sender;
use std::thread;

use crate::blockchain::client_event::{ClientEvent, ClientEventReader};
use std::io::Read;

#[derive(Debug)]
pub struct InputHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl InputHandler {
    pub fn new<T: 'static + Read + Send>(source: T, input_sender: Sender<ClientEvent>) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            InputHandler::run(source, input_sender).unwrap();
        }));
        InputHandler { thread_handle }
    }

    fn run<T: Read>(source: T, input_sender: Sender<ClientEvent>) -> io::Result<()> {
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
