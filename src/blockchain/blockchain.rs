use std::u8;

#[derive(Debug, Clone)]
pub struct Transaction {}

#[derive(Debug, Clone)]
pub struct Blockchain {}

impl Blockchain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn add_transaction(&mut self, _transaction: Transaction) {}

    pub fn refresh(&mut self) {}

    pub fn send_result() {}

    pub fn as_bytes(&self) -> &'static [u8] {
        todo!()
    }
    pub fn validate(&self, _tra: Transaction) -> bool {
        todo!()
    }
}
