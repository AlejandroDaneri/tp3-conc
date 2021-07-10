use std::process;
use tp3::blockchain::client::Client;

fn main() -> std::io::Result<()> {
    let port_from = 9000;
    let port_to = 9003;
    let id = process::id();
    let client = Client::new(id);
    client.run(port_from, port_to)
}
