use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::blockchain::client_event::Message::Common;
use crate::blockchain::client_event::{
    ClientEvent, ClientEventReader, ClientMessage, ErrorMessage,
};
use std::io::Read;

#[derive(Debug)]
pub struct InputHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl InputHandler {
    pub fn new<T: 'static + Read + Send>(
        source: T,
        input_sender: Sender<ClientEvent>,
        output_receiver: Receiver<ClientMessage>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            InputHandler::run(source, input_sender, output_receiver).unwrap();
        }));
        InputHandler { thread_handle }
    }

    fn run<T: Read>(
        source: T,
        input_sender: Sender<ClientEvent>,
        output_receiver: Receiver<ClientMessage>,
    ) -> io::Result<()> {
        let message_reader = ClientEventReader::new(source);
        for message in message_reader {
            let event = ClientEvent::UserInput {
                message: message.clone(),
            };
            input_sender.send(event).unwrap();
            println!("Waiting response...");
            let response = output_receiver.recv().unwrap();
            println!("Response: {:?}", response);
            match response {
                ClientMessage::ErrorResponse(ErrorMessage::LockNotAcquiredError) => {
                    let mut lock_acquired = false;
                    while !lock_acquired {
                        let lock_msg = ClientMessage::LockRequest { read_only: true };
                        println!("Asking lock...");
                        input_sender.send(ClientEvent::UserInput {
                            message: Common(lock_msg),
                        });
                        let response = output_receiver.recv().unwrap();
                        println!("Response: {:?}", response);
                        if let ClientMessage::LockResponse { acquired } = response {
                            lock_acquired = acquired;
                        }
                    }
                    let event = ClientEvent::UserInput { message };
                    input_sender.send(event);
                    let response = output_receiver.recv().unwrap();
                    println!("Response: {:?}", response);
                }
                ClientMessage::ErrorResponse(ErrorMessage::NotLeaderError) => {
                    println!(
                        "TODO: Ocurrió {:?}, realizar un leader request automático.",
                        response
                    )
                }
                ClientMessage::ReadBlockchainResponse {blockchain} => {
                    println!("Blockchain: {}", blockchain);
                }
                _ => println!("{:?}", response),
            }
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
