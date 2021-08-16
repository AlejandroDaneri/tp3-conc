use crate::communication::client_event::{ClientEvent, ClientMessage, ErrorMessage};
use crate::communication::client_event::{LockMessage, Message};
use crate::communication::commands::UserCommand;
use crate::communication::dispatcher::Dispatcher;
use crate::communication::serialization::LineReader;
use std::io::Read;
use std::sync::mpsc::Receiver;

pub struct InputProcessor {
    output_receiver: Receiver<ClientMessage>,
    dispatcher: Dispatcher,
}

impl InputProcessor {
    pub fn new(output_receiver: Receiver<ClientMessage>, dispatcher: Dispatcher) -> Self {
        InputProcessor {
            output_receiver,
            dispatcher,
        }
    }

    pub fn run<R: Read>(&self, source: R) {
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
                    self.dispatcher.dispatch(event).ok();
                    status = ClientStatus::WaitingReply;
                }
                ClientStatus::WaitingReply => {
                    info!("Waiting reply...");
                    let response = self.output_receiver.recv().unwrap();
                    info!("Input handler response -> {:?}", response);
                    match response {
                        ClientMessage::ErrorResponse(ErrorMessage::LockNotAcquiredError) => {
                            let event = ClientEvent::UserInput {
                                message: Message::Lock(LockMessage::Acquire),
                            };
                            self.dispatcher.dispatch(event).ok();
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
                        ClientMessage::WriteBlockchainResponse { .. } => {
                            if let Some(UserCommand::WriteBlockchain(_)) = current_command {
                                info!("Write blockchain exitoso");
                                status = ClientStatus::Idle;
                            }
                            let event = ClientEvent::UserInput {
                                message: Message::Lock(LockMessage::Release),
                            };
                            self.dispatcher.dispatch(event).ok();
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
                            self.dispatcher.dispatch(event).ok();
                        }
                        ClientMessage::ErrorResponse(error) => {
                            error!("Error: {:?}", error);
                            error!("Retrying....");
                            status = ClientStatus::SendCommand;
                        }
                    }
                }
            }
        }
        error!("Saliendo de la aplicación");
    }
}
enum ClientStatus {
    Idle,
    SendCommand,
    WaitingReply,
}
