use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
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
    Insert(TransactionData),
    Remove(String),
}

impl Transaction {
    pub fn is_valid(&self) -> bool {
        match self {
            Transaction::Insert(data) => data.is_valid(),
            Transaction::Remove(key) => !key.is_empty(),
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            Transaction::Insert(data) => format!("insert {}", data.serialize()),
            Transaction::Remove(key) => format!("remove {}", key),
        }
    }

    pub fn parse(tokens: &mut dyn Iterator<Item = &str>) -> Option<Self> {
        let action = tokens.next();
        match action {
            Some("insert") => Some(Transaction::Insert(TransactionData::parse(tokens)?)),
            Some("remove") => {
                let name = tokens.next()?;
                Some(Transaction::Remove(name.to_owned()))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionData {
    student: String,
    score: u16,
}

impl TransactionData {
    pub fn new(student: &str, score: u16) -> Self {
        let student = student.to_owned();
        Self { student, score }
    }

    pub fn serialize(&self) -> String {
        format!("{} {}", self.student, self.score)
    }

    pub fn parse(tokens: &mut dyn Iterator<Item = &str>) -> Option<Self> {
        let student = tokens.next()?;
        let score = tokens.next()?;
        Some(Self::new(
            student,
            u16::from_str(score).expect("invalid score number"),
        ))
    }

    pub fn is_valid(&self) -> bool {
        self.score > 0 && self.score <= 10
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    transaction: Transaction,
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

    pub fn serialize(&self) -> String {
        format!(
            "{} {}",
            self.transaction.serialize(),
            format!("{}", self.previous_hash),
        )
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
        }
        Transaction::Remove(key) => {
            hasher.write(key.as_bytes());
        }
    };
    hasher.write_u64(previous_hash);
    hasher.finish()
}
#[derive(Debug, Clone)]
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
    pub fn get_last(&self) -> Option<&Block> {
        self.blocks.last()
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {
        let prev_hash = if let Some(last) = self.get_last() {
            last.hash
        } else {
            0
        };
        let block = Block::new(transaction, prev_hash).unwrap();
        self.add_block(block);
    }
    pub fn validate(&self, _transaction: &Transaction) -> bool {
        true
    }

    pub fn serialize(&self) -> String {
        let mut response = String::new();
        for block in self.blocks.iter() {
            if response.is_empty() {
                response = block.serialize()
            } else {
                response = format!("{} {}", response, block.serialize())
            }
        }
        format!("{} end_blockchain", response)
    }

    pub fn parse(tokens: &mut dyn Iterator<Item = &str>) -> Option<Self> {
        let mut blockchain = Blockchain::new();
        while let Some(transaction) = Transaction::parse(tokens) {
            let previous_hash = tokens.next()?;
            let block = Block::new(transaction, previous_hash.parse::<u64>().ok()?).ok()?;
            blockchain.add_block(block);
        }
        Some(blockchain)
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries = HashMap::new();
        for block in &self.blocks {
            match &block.transaction {
                Transaction::Insert(data) => {
                    let key = data.student.clone();
                    entries.insert(key, data.score);
                }
                Transaction::Remove(student) => {
                    entries.remove(student);
                }
            }
        }
        for (student, score) in entries.iter() {
            writeln!(f, "Student {} -> {}", student, score)?;
        }
        Ok(())
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
        let last_block = bc.get_last().unwrap();
        assert_eq!(*last_block, block);
    }

    #[test]
    fn parse_transaction() {
        let transaction_str = "insert pedro 10 0 insert juan 2 123";
        let mut tokens = transaction_str.split_whitespace();
        let transaction = Transaction::parse(&mut tokens);
        let prev_hash = tokens.next();
        let transaction_2 = Transaction::parse(&mut tokens);
        assert!(transaction.is_some());
        assert!(transaction_2.is_some());
    }

    #[test]
    fn parse_blockchain() {
        let blockchain_str =
            "blockchain insert pedro 10 0 123 insert juan 2 123 345 end_blockchain";
        let mut tokens = blockchain_str.split_whitespace().skip(1);
        let blockchain = Blockchain::parse(&mut tokens);
        assert!(blockchain.is_some());
    }
}
