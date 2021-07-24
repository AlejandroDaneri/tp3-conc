use std::collections::hash_map::DefaultHasher;

use std::hash::Hasher;

use std::fmt;

#[derive(Debug)]
pub enum BlockError {
    CreateError,
}

impl std::error::Error for BlockError {}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockError::CreateError => write!(f, "HTTP Error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    student: String,
    score: u16,
}

impl Transaction {
    pub fn new(student: String, score: u16) -> Self {
        Self { student, score }
    }

    pub fn is_valid(&self) -> bool {
        //validamos estudiante de alguna forma?
        self.score > 0 && self.score <= 10
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    transaction: Transaction, // podria ser mas de una transaction x bloque en la realidad
    previous_hash: u64,
    hash: u64,
}

impl Block {
    pub fn new(transaction: Transaction, previous_hash: u64) -> Result<Self, BlockError> {
        if !transaction.is_valid() {
            return Err(BlockError::CreateError);
        };
        let hash = generate_hash(transaction.clone(), previous_hash);
        Ok(Self {
            transaction,
            previous_hash,
            hash,
        })
    }

    pub fn is_valid(&self) -> bool {
        if !self.transaction.is_valid() {
            return false;
        }
        self.hash == hash_block(self.clone())
    }
}

fn hash_block(record: Block) -> u64 {
    generate_hash(record.transaction, record.previous_hash)
}

fn generate_hash(transaction: Transaction, previous_hash: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(transaction.student.as_bytes());
    hasher.write_u16(transaction.score);
    hasher.write_u64(previous_hash);
    hasher.finish()
}
#[derive(Debug)]
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block)
    }
    pub fn get_last(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    //temporal para que compile
    pub fn as_bytes(&self) {}
    pub fn add_transaction(&self, transaction: Transaction) {}
    pub fn validate(&self, transaction: Transaction) -> bool {
        true
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eql_block(block1: &Block, block2: Block) -> bool {
        block1.hash == block2.hash
            && block1.previous_hash == block2.previous_hash
            && block1.transaction.score == block2.transaction.score
            && block1.transaction.student == block2.transaction.student
    }

    #[test]
    fn test_valid_transaction() {
        let transaction = Transaction::new("Pedro".to_string(), 10);
        assert_eq!(transaction.is_valid(), true)
    }

    #[test]
    fn test_invalid_transaction_greater_than_max() {
        let transaction = Transaction::new("Pedro".to_string(), u16::MAX);
        assert_eq!(transaction.is_valid(), false)
    }

    #[test]
    fn first_block_must_be_valid() {
        let transaction = Transaction::new("Pedro".to_string(), 7);
        let block = Block::new(transaction, 0).unwrap();
        assert_eq!(block.is_valid(), true);
    }
    #[test]
    fn block_with_invalid_transaction_should_be_not_created() {
        let transaction = Transaction::new("Pedro".to_string(), u16::MAX);
        let block = Block::new(transaction, 0);
        assert!(block.is_err());
    }

    #[test]
    fn different_blocks_with_same_content_must_be_eql() {
        let transaction = Transaction::new("Pedro".to_string(), 7);
        let block = Block::new(transaction.clone(), 1245).unwrap();
        let block2 = Block::new(transaction, 1245).unwrap();
        assert_eq!(block.hash, block2.hash);
    }

    #[test]
    fn block_must_be_the_same_after_adding_to_bc() {
        let transaction = Transaction::new("Pedro".to_string(), 7);
        let mut bc = Blockchain::new();
        let block = Block::new(transaction, 0).unwrap();
        bc.add_block(block.clone());
        let last_block = bc.get_last();
        assert_eq!(eql_block(last_block, block), true);
    }
}
