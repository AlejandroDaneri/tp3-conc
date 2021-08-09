use std::io;
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::communication::client_event::{Message, LockMessage};
use crate::communication::client_event::{ClientEvent, ClientMessage, ErrorMessage};
use crate::communication::commands::UserCommand;
use crate::communication::serialization::LineReader;

#[derive(Debug)]
pub struct InputHandler {
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl InputHandler {
    pub fn new<R: 'static + Read + Send>(
        source: R,
        input_sender: Sender<ClientEvent>,
        output_receiver: Receiver<ClientMessage>,
    ) -> Self {
        let thread_handle = Some(thread::spawn(move || {
            InputHandler::run(source, input_sender, output_receiver);
        }));
        InputHandler { thread_handle }
    }

    fn run<R: 'static + Read + Send>(
        source: R,
        message_sender: Sender<ClientEvent>,
        output_receiver: Receiver<ClientMessage>,
    ) {
        let processor = InputProcessor::new(message_sender, output_receiver);
        processor.run(source).unwrap();
    }
}

impl Drop for InputHandler {
    fn drop(&mut self) {
        let _ = self.thread_handle.take().unwrap().join();
    }
}

pub struct InputProcessor {
    message_sender: Sender<ClientEvent>,
    output_receiver: Receiver<ClientMessage>,
}

impl InputProcessor {
    pub fn new(
        message_sender: Sender<ClientEvent>,
        output_receiver: Receiver<ClientMessage>,
    ) -> Self {
        InputProcessor {
            message_sender,
            output_receiver,
        }
    }

    fn run<R: Read>(&self, source: R) -> io::Result<()> {
        let command_reader = LineReader::<R, UserCommand>::new(source);
        for command in command_reader {
            self.handle_command(&command);
        }
        println!("Saliendo de la aplicación");
        Ok(())
    }

    fn handle_command(&self, command: &UserCommand) {
        let message;
        match command {
            UserCommand::ReadBlockchain => {
                message = Message::Common(ClientMessage::ReadBlockchainRequest)
            }
            UserCommand::WriteBlockchain(transaction) => {
                message = Message::Common(ClientMessage::WriteBlockchainRequest {
                    transaction: transaction.clone(),
                })
            }
            _ => {
                return;
            }
        }
        let event = ClientEvent::UserInput {
            message: message.clone(),
        };
        self.message_sender.send(event);
        let mut response;
        loop {
            println!("Waiting response...");
            response = self.output_receiver.recv().unwrap();
            match response {
                ClientMessage::ErrorResponse(ErrorMessage::LockNotAcquiredError) => {
                    let event = ClientEvent::UserInput {
                        message: Message::Lock(LockMessage::Acquire),
                    };
                    self.message_sender.send(event);
                }
                ClientMessage::LockResponse (true) => {
                    println!("Lock acquired! retrying... ");
                    let event = ClientEvent::UserInput {
                        message: message.clone(),
                    };
                    self.message_sender.send(event);
                }
                ClientMessage::ReadBlockchainResponse { .. }
                | ClientMessage::WriteBlockchainResponse { .. } => {
                    break;
                }
                _ => {
                    let event = ClientEvent::UserInput {
                        message: message.clone(),
                    };
                    self.message_sender.send(event);
                    println!("Retrying after {:?}", response);
                }
            }
        }
        println!("Response: {:?}", response);
    }
}
