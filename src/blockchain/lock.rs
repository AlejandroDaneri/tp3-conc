use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

pub trait Lock {
    // fn acquire(&mut self, read_only: bool, stream: &mut TcpStream); para que compile
    fn acquire(&mut self);

    // fn release(&mut self, stream: &mut TcpStream); para que compile
    fn release(&mut self);
}

#[derive(Debug)]
pub struct CentralizedLock {
    coordinator_stream: TcpStream,
    stream_reader: BufReader<TcpStream>,
}

impl Lock for CentralizedLock {
    //envio pedido de acquire al coordinador
    fn acquire(&mut self) {
        self.coordinator_stream
            .write_all(ACQUIRE_MSG.as_bytes())
            .unwrap();

        let mut buffer = String::new();
        self.stream_reader.read_line(&mut buffer); // devuelve respuesta del coordinador
    }

    //envio pedido de release al coordinador
    fn release(&mut self) {
        self.coordinator_stream
            .write_all(RELEASE_MSG.as_bytes())
            .unwrap();
    }
}

const ACQUIRE_MSG: &str = "acquire\n";

const RELEASE_MSG: &str = "release\n";

impl CentralizedLock {
    pub fn new(id: u32, coordinator_stream: TcpStream) -> CentralizedLock {
        let mut new_lock = CentralizedLock {
            coordinator_stream: {
                match coordinator_stream.try_clone() {
                    Ok(stream) => stream,
                    Err(_e) => todo!(),
                }
            },
            stream_reader: BufReader::new(coordinator_stream),
        };

        new_lock
            .coordinator_stream
            .write_all((id.to_string() + "\n").as_bytes());

        new_lock
    }
}
