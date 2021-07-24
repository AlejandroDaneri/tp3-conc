use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::Hasher;
use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq)]
pub enum Transaction {
    Insert(TransactionData), Remove(String)
}

impl Transaction {
    pub fn is_valid(&self) -> bool {
        match self {
            Transaction::Insert(data) => data.is_valid(),
            Transaction::Remove(key) => !key.is_empty(),
        }
    }

    pub fn parse(tokens: &mut dyn Iterator<Item = &str>) -> Option<Self> {
        let action = tokens.next();
        match action {
            Some("insert") => {
                Some(Transaction::Insert(TransactionData::parse(tokens)?))
            },
            _ => None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionData {
    student: String,
    score: u16
}

impl TransactionData {
    pub fn new(student: &str, score: u16) -> Self {
        let student = student.to_owned();
        Self { student, score }
    }

    pub fn parse(tokens: &mut dyn Iterator<Item = &str>) -> Option<Self> {
        let student = tokens.next()?;
        let score = tokens.next()?;
        Some(Self::new(student, u16::from_str(score).expect("invalid score number")))
    }

    pub fn is_valid(&self) -> bool {
        //validamos estudiante de alguna forma?
        self.score > 0 && self.score <= 10
    }
}

#[derive(Debug, Clone, PartialEq)]
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
        self.transaction.is_valid() && self.hash == hash_block(self.clone())
    }
}

fn hash_block(record: Block) -> u64 {
    generate_hash(record.transaction, record.previous_hash)
}

fn generate_hash(transaction: Transaction, previous_hash: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    match transaction {
        Transaction::Insert(data) => {
            hasher.write(data.student.as_bytes());
            hasher.write_u16(data.score);
        },
        Transaction::Remove(key) => {hasher.write(key.as_bytes());},
    };
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
    pub fn as_str(&self) -> String {
        "Blockchain".to_owned()
    }
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

    #[test]
    fn test_valid_transaction() {
        let transaction_data = TransactionData::new("Pedro", 10);
        assert_eq!(transaction_data.is_valid(), true)
    }

    #[test]
    fn test_invalid_transaction_greater_than_max() {
        let transaction_data = TransactionData::new("Pedro", u16::MAX);
        assert_eq!(transaction_data.is_valid(), false)
    }

    #[test]
    fn first_block_must_be_valid() {
        let transaction_data = TransactionData::new("Pedro", 7);
        let transaction = Transaction::Insert(transaction_data);
        let block = Block::new(transaction, 0).unwrap();
        assert_eq!(block.is_valid(), true);
    }
    #[test]
    fn block_with_invalid_transaction_should_be_not_created() {
        let transaction_data = TransactionData::new("Pedro", u16::MAX);
        let transaction = Transaction::Insert(transaction_data);
        let block = Block::new(transaction, 0);
        assert!(block.is_err());
    }

    #[test]
    fn different_blocks_with_same_content_must_be_eql() {
        let transaction_data = TransactionData::new("Pedro", 7);
        let transaction = Transaction::Insert(transaction_data);
        let block = Block::new(transaction.clone(), 1245).unwrap();
        let block2 = Block::new(transaction, 1245).unwrap();
        assert_eq!(block.hash, block2.hash);
    }

    #[test]
    fn block_must_be_the_same_after_adding_to_bc() {
        let transaction_data = TransactionData::new("Pedro", 7);
        let transaction = Transaction::Insert(transaction_data);
        let mut bc = Blockchain::new();
        let block = Block::new(transaction, 0).unwrap();
        bc.add_block(block.clone());
        let last_block = bc.get_last();
        assert_eq!(*last_block, block);
    }
}
