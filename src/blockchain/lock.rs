use std::net::TcpStream;

pub trait Lock {
    fn acquire(&mut self, read_only: bool, stream: &mut TcpStream);

    fn release(&mut self, stream: &mut TcpStream);
}

#[derive(Debug)]
pub struct CentralizedLock {}

impl Lock for CentralizedLock {
    fn acquire(&mut self, read_only: bool, stream: &mut TcpStream) {}

    fn release(&mut self, stream: &mut TcpStream) {}
}

impl CentralizedLock {
    pub fn new() -> Self {
        Self {}
    }
}
