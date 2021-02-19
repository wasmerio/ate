extern crate tokio;

use std::{net::SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use super::conf::*;

#[allow(unused_imports)]
use tokio::runtime::Runtime;

pub struct Server {
    
}

#[allow(dead_code)]
impl Server {
    fn new() -> Server {
        Server {
        }
    }

    pub async fn run(&self, conf: &impl ConfigMaster) -> io::Result<()> {
        let addr = format!("{}:{}", conf.master_addr(), conf.master_port());
        let listener = TcpListener::bind(addr).await.unwrap();
        loop {
            let (stream, sock_addr) = listener.accept().await.unwrap();
            setup_tcp_stream(&stream)?;

            tokio::spawn(async move {
                process_steam(stream, sock_addr).await;
            });
        }
    }
}

#[allow(unused_variables)]
async fn process_steam(mut stream: TcpStream, sock_addr: SocketAddr) -> () {
    let (mut rd, mut wr) = stream.split();

    let mut buf: Vec<u8> = vec![0; 128];
    loop {
        let n = rd.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }

        println!("GOT {:?}", &buf[..n]);
        wr.write_all(b"nice to meet you\n").await.unwrap();
    }
}

pub struct Client {

}

#[allow(dead_code)]
impl Client {
    fn new() -> Client {
        Client {
        }
    }

    pub async fn run(&self, conf: &impl ConfigMaster) -> io::Result<()> {
        let addr = format!("{}:{}", conf.master_addr(), conf.master_port());
        
        let mut stream = TcpStream::connect(addr).await.unwrap();
        setup_tcp_stream(&stream)?;
        
        let (mut rd, mut wr) = stream.split();
        wr.write_all(b"hi\n").await?;
        
        let mut buf: Vec<u8> = vec![0; 128];
        
        let n = rd.read(&mut buf).await.unwrap();
        if n == 0 {
            panic!("Failed to receive the correct data");
        }
        println!("GOT {:?}", &buf[..n]);
        
        assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "nice to meet you\n");

        Ok(())
    }
}

fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[test]
fn test_server_client() {
    let rt = Runtime::new().unwrap();

    let cfg = super::conf::mock_test_config();
    rt.spawn(async move {
        let server = Server::new();
        server.run(&cfg).await.expect("The networking server has panicked");
    });

    let cfg = super::conf::mock_test_config();
    rt.block_on(async move {
        let client = Client::new();
        client.run(&cfg).await.expect("The networking client has panicked");
    });
}