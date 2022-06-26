use std::net::TcpStream;

fn main() {
    match TcpStream::connect("8.8.8.8:53") {
        Ok(_stream) => {
            println!("Successfully connected to server in port 53");
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
    println!("Finished.");
}
