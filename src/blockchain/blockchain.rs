#[derive(Debug)]
pub struct Transaction {}

#[derive(Debug)]
pub struct Blockchain {}

impl Blockchain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {}

    pub fn refresh(&mut self) {}

    pub fn send_result() {}
}
