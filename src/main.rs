use blockchain::blockchain::client::Client;
use std::io;
use std::process;

fn main() -> std::io::Result<()> {
    let port_from = 9000;
    let port_to = 9010;
    let id = process::id();
    let mut client = Client::new(id);
    client.run(io::stdin(), port_from, port_to)
}
