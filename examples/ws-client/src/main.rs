use wasm_bus_ws::prelude::*;

fn main() {
    println!("creating web socket and opening");
    let ws = SocketBuilder::new_str("wss://ws.postman-echo.com/raw")
        .blocking_open()
        .unwrap();

    let data = vec![ 1u8, 2u8, 3u8 ];
    
    println!("sending data to socket");
    let (tx, mut rx) = ws.split();
    tx.blocking_send(data).unwrap();

    println!("receiving data from socket");
    let test = rx.blocking_recv();

    assert!(test == Some(vec![ 1u8, 2u8, 3u8 ]), "data is not the same");
    println!("success");
}
