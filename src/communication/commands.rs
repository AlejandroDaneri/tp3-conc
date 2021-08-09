use crate::blockchain::blockchain::Transaction;
use crate::communication::serialization::Serializable;

#[derive(Clone, Debug)]
pub enum UserCommand {
    Exit,
    ReadBlockchain,
    WriteBlockchain(Transaction),
}

impl Serializable for UserCommand {
    fn serialize(&self) -> String {
        unreachable!()
    }

    fn deserialize(line: &str) -> Option<Self> {
        let mut tokens = line.split_whitespace();
        let action = tokens.next();
        match action {
            Some("rb") => Some(UserCommand::ReadBlockchain),
            Some("wb") => {
                let transaction = Transaction::parse(&mut tokens)?;
                Some(UserCommand::WriteBlockchain(transaction))
            }
            Some("exit") => Some(UserCommand::Exit {}),
            _ => None,
        }
    }
}
