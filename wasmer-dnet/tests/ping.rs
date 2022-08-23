#![allow(unused_variables)]
use std::net::IpAddr;
use std::net::Ipv4Addr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use serial_test::serial;
use std::time::Instant;
use std::convert::TryInto;
use byteorder::WriteBytesExt;
use byteorder::LittleEndian;

mod common;

#[test]
#[serial]
fn ping_static() {
    run_ping(false);
}

#[test]
#[serial]
fn ping_dhcp() {
    run_ping(true);
}

fn create_ping_packet(start: Instant, seq: u64) -> std::io::Result<Vec<u8>>
{
    let now = Instant::now();

    let mut pck = vec![0u8; 36];
    let mut buf = &mut pck[..];
    buf.write_u8(8)?;   // type (1-byte)
    buf.write_u8(0)?;   // code (1-byte)
    buf.write_u16::<LittleEndian>(0)?;  // checksum (2-bytes)
    buf.write_u128::<LittleEndian>(now.duration_since(start).as_nanos())?;   // timestamp (16-bytes)
    buf.write_u64::<LittleEndian>(seq)?;    // seq (8-bytes)
    buf.write_u64::<LittleEndian>(fastrand::u64(..))?; // random bytes (8-bytes)

    calculate_checksum(&mut pck[..]);
    Ok(pck)
}

fn calculate_checksum(data: &mut [u8]) {
    let mut f = 0;
    let mut chk: u32 = 0;
    while f + 2 <= data.len() {
        chk += u16::from_le_bytes(data[f..f+2].try_into().unwrap()) as u32;

        f += 2;
    }

    while chk > 0xffff {
        chk = (chk & 0xffff) + (chk >> 2*8);
    }

    let mut chk = chk as u16;
    chk = !chk & 0xffff;
    data[3] = (chk >> 8) as u8;
    data[2] = (chk & 0xff) as u8;
}

fn run_ping(use_dhcp: bool) {
    common::run(async move {
        let _servers = common::setup().await;

        let (c1, _) = common::clients(false, use_dhcp).await;
        let s1_addr = IpAddr::V4(c1.addr_ipv4().await.unwrap().unwrap());
        
        let ident = 0x22b;
        let s1 = c1.bind_icmp(s1_addr).await.unwrap();
        
        let start = Instant::now();
        for seq in 0..10 {
            let buf = create_ping_packet(start, seq).unwrap();
            s1.send_to(buf, IpAddr::V4(Ipv4Addr::new(10, 35, 196, 1))).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
}