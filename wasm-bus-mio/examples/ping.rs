use std::collections::HashMap;
use std::time::Instant;
use std::time::Duration;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::convert::TryInto;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use byteorder::LittleEndian;

use wasm_bus_mio::mio::AsyncIcmpSocket;

use clap::Parser;

#[repr(C)]
struct IcmpPacket
{
    ty: u8,
    code: u8,
    checksum: u16,
    timestamp: u128,
    seq: u64,
    rand: u64,
}

fn create_ping_packet(destination: IpAddr, start: Instant, seq: u64) -> std::io::Result<Vec<u8>>
{
    let now = Instant::now();

    let icmp = IcmpPacket {
        ty: match &destination {
            IpAddr::V4(_) => 1,
            IpAddr::V6(_) => 3,
        },
        code: 8,
        checksum: 0,
        timestamp: now.duration_since(start).as_nanos(),
        seq,
        rand: fastrand::u64(..)
    };

    let mut pck = vec![0u8; ::std::mem::size_of::<IcmpPacket>()];
    let mut buf = &mut pck[..];
    buf.write_u8(icmp.ty)?;
    buf.write_u8(icmp.code)?;
    buf.write_u16::<LittleEndian>(icmp.checksum)?;
    buf.write_u128::<LittleEndian>(icmp.timestamp)?;
    buf.write_u64::<LittleEndian>(icmp.seq)?;
    buf.write_u64::<LittleEndian>(icmp.rand)?;

    calculate_checksum(&mut pck[..]);
    Ok(pck)
}

fn decode_pong_packet(pck: Vec<u8>) -> std::io::Result<IcmpPacket> {
    let mut buf = &pck[..];
    Ok(
        IcmpPacket {
            ty: buf.read_u8()?,
            code: buf.read_u8()?,
            checksum: buf.read_u16::<LittleEndian>()?,
            timestamp: buf.read_u128::<LittleEndian>()?,
            seq: buf.read_u64::<LittleEndian>()?,
            rand: buf.read_u64::<LittleEndian>()?,
        }
    )
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

fn check_checksum(data: &mut [u8]) -> bool {
    let d1 = data[2];
    let d2 = data[3];
    data[2] = 0;
    data[3] = 0;

    calculate_checksum(data);
    d1 == data[2] && d2 == data[3]
}

#[derive(Parser)]
#[clap(version = "1.0", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Amount of echo request packets to send
    #[allow(dead_code)]
    #[clap(short, long, default_value = "4")]
    pub count: i32,
    /// Interval between successive packets sent (milliseconds)
    #[allow(dead_code)]
    #[clap(short, long, default_value = "1000")]
    pub interval: u64,
    //Maximum wait duration for an echo response packet (milliseconds)
    #[allow(dead_code)]
    #[clap(short, long, default_value = "5000")]
    pub timeout: u64,
    // IP address of destination to ping
    #[allow(dead_code)]
    #[clap(index = 1)]
    pub destination: IpAddr,
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasm_bus::task::block_on(main_async())
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    main_async().await?;
    std::process::exit(0);
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    
    let destination = opts.destination;
    let count = opts.count as u64;
    let interval = Duration::from_millis(opts.interval);
    let timeout = Duration::from_millis(opts.timeout);

    let source = match &destination {
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };

    let mut dups = HashMap::<u64, u32>::new();
    let mut sent = 0u64;
    let mut received = 0u64;

    println!("PING {} ({}) 56(84) bytes of data", destination, destination);

    let start = Instant::now();
    let socket = AsyncIcmpSocket::bind(source).await?;
    for seq in 0..count
    {
        let pck = create_ping_packet(destination, start, seq)?;
        socket.send_to(pck, destination).await?;
        sent += 1;

        let wait = if sent < count {
            interval
        } else {
            timeout
        };

        let now = Instant::now();
        while now - start < wait {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(10)) => { },
                ret = socket.recv_from(1024) => {
                    let (mut pck, from) = ret?;
                    let pck_len = pck.len();
                    let checksum_ok = check_checksum(&mut pck[..]);
                    match decode_pong_packet(pck) {
                        Ok(pck) => {
                            let seq = pck.seq;
                            let duration = Duration::from_nanos(pck.timestamp as u64);
                            let duration = match Instant::now().duration_since(start).checked_sub(duration) {
                                Some(a) => a,
                                None => Duration::ZERO
                            };
                            print!("{} bytes from {}: icmp_seq={} ttl=64, time={}ms", pck_len, from, pck.seq, duration.as_millis());

                            let cur = dups.get(&seq).map(|a| a.clone()).unwrap_or(0u32);
                            dups.insert(seq, cur + 1);
                            
                            if cur <= 0 {
                                received += 1;
                            } else if cur == 1 {
                                print!(" (DUP!)");
                            } else {
                                print!(" (DUP![{}])", cur);
                            }

                            if checksum_ok {
                                println!("")
                            } else {
                                println!(" invalid checksum!");
                            }
                        },
                        Err(_) => {
                            println!("{} bytes from {}: invalid packet data", pck_len, from);
                        }
                    }        
                }   
            }
        }
    }

    println!("--- {} ping statistics ---", destination);
    if sent > 0 {
        println!(
            "{} packets transmitted, {} received, {:.0}% packet loss, time {}ms",
            sent,
            received,
            100.0 * (sent - received) as f64 / sent as f64,
            Instant::now().duration_since(start).as_millis()
        );
    }
    Ok(())
}