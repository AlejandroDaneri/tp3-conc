use crate::communication::client_event::{ClientEvent, ClientMessage, ErrorMessage};
use crate::communication::client_event::{LockMessage, Message};
use crate::communication::commands::UserCommand;
use crate::communication::serialization::LineReader;
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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
        processor.run(source);
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
    fn run<R: Read>(&self, source: R) {
        let mut command_reader = LineReader::<R, UserCommand>::new(source);
        let mut status = ClientStatus::Idle;
        let mut current_command = None;
        loop {
            match status {
                ClientStatus::Idle => {
                    println!("Ingrese un comando");
                    current_command = command_reader.next();
                    status = ClientStatus::SendCommand;
                }
                ClientStatus::SendCommand => {
                    let message;
                    match &current_command {
                        Some(UserCommand::ReadBlockchain) => {
                            message = Message::Common(ClientMessage::ReadBlockchainRequest)
                        }
                        Some(UserCommand::WriteBlockchain(transaction)) => {
                            message = Message::Common(ClientMessage::WriteBlockchainRequest {
                                transaction: transaction.clone(),
                            })
                        }
                        _ => {
                            break;
                        }
                    }
                    let event = ClientEvent::UserInput { message };
                    self.message_sender.send(event).ok();
                    status = ClientStatus::WaitingReply;
                }
                ClientStatus::WaitingReply => {
                    println!("Waiting reply");
                    let response = self.output_receiver.recv().unwrap();
                    println!("---- input_handler response -> {:?}", response);
                    match response {
                        ClientMessage::ErrorResponse(ErrorMessage::LockNotAcquiredError) => {
                            let event = ClientEvent::UserInput {
                                message: Message::Lock(LockMessage::Acquire),
                            };
                            self.message_sender.send(event).ok();
                        }
                        ClientMessage::LockResponse(true) => {
                            status = ClientStatus::SendCommand;
                        }
                        ClientMessage::ReadBlockchainResponse { blockchain } => {
                            if let Some(UserCommand::ReadBlockchain) = current_command {
                                println!("Blockchain: {}", blockchain);
                                status = ClientStatus::Idle;
                            }
                        }
                        ClientMessage::WriteBlockchainResponse {} => {
                            if let Some(UserCommand::WriteBlockchain(_)) = current_command {
                                println!("Write blockchain exitoso");
                                status = ClientStatus::Idle;
                            }
                            let event = ClientEvent::UserInput {
                                message: Message::Lock(LockMessage::Release),
                            };
                            self.message_sender.send(event).ok();
                        }
                        ClientMessage::ReadBlockchainRequest => todo!(),
                        ClientMessage::WriteBlockchainRequest { transaction: _ } => todo!(),
                        ClientMessage::LeaderElectionFinished => {
                            status = ClientStatus::SendCommand;
                        }
                        ClientMessage::BroadcastBlockchain { blockchain: _ } => {}
                        ClientMessage::LockResponse(_acquired) => {
                            let event = ClientEvent::UserInput {
                                message: Message::Lock(LockMessage::Acquire),
                            };
                            self.message_sender.send(event).ok();
                        }
                        ClientMessage::ErrorResponse(error) => {
                            println!("Error: {:?}", error);
                            println!("Retrying....");
                            status = ClientStatus::SendCommand;
                        }
                    }
                }
            }
        }
        println!("Saliendo de la aplicación");
    }
}
enum ClientStatus {
    Idle,
    SendCommand,
    WaitingReply,
}
