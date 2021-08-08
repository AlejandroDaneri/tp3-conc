use std::io::{BufRead, BufReader, Read};
use std::marker::PhantomData;

pub trait Serializable {
    fn serialize(&self) -> String;
    fn deserialize(line: &str) -> Option<Self>
    where
        Self: Sized;
}

pub struct LineReader<R, O> {
    reader: BufReader<R>,
    _phantom: PhantomData<O>,
}

impl<R: Read, O> LineReader<R, O> {
    pub fn new(source: R) -> Self {
        let reader = BufReader::new(source);
        let _phantom = PhantomData;
        Self { reader, _phantom }
    }
}

impl<R: Read, O> Iterator for LineReader<R, O>
where
    O: Serializable,
{
    type Item = O;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut line = String::new();
        self.reader.read_line(&mut line).ok()?;
        line.pop();
        O::deserialize(&line)
    }
}
