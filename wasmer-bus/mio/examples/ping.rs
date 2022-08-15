use std::collections::HashMap;
use std::time::Instant;
use std::time::Duration;
use std::net::IpAddr;
use std::convert::TryInto;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use byteorder::LittleEndian;

use wasmer_bus_mio::prelude::*;
#[cfg(target_family = "wasm")]
use wasmer_bus_time::prelude::sleep;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

use clap::Parser;

#[repr(C)]
struct IcmpHeader
{
    ty: u8,
    code: u8,
    checksum: u16,
}

#[repr(C)]
struct IcmpPingPong
{
    timestamp: u128,
    seq: u64,
    rand: u64,
}

#[repr(C)]
struct IcmpPing
{
    header: IcmpHeader,
    ping: IcmpPingPong
}

fn create_ping_packet(start: Instant, seq: u64) -> std::io::Result<Vec<u8>>
{
    let now = Instant::now();

    let icmp = IcmpPing {
        header: IcmpHeader {
            ty: 8,  //IcmpType::Echo
            code: 0,
            checksum: 0,
        },
        ping: IcmpPingPong {
            timestamp: now.duration_since(start).as_nanos(),
            seq,
            rand: fastrand::u64(..)
        }
    };

    let mut pck = vec![0u8; ::std::mem::size_of::<IcmpPing>()];
    let mut buf = &mut pck[..];
    buf.write_u8(icmp.header.ty)?;
    buf.write_u8(icmp.header.code)?;
    buf.write_u16::<LittleEndian>(icmp.header.checksum)?;
    buf.write_u128::<LittleEndian>(icmp.ping.timestamp)?;
    buf.write_u64::<LittleEndian>(icmp.ping.seq)?;
    buf.write_u64::<LittleEndian>(icmp.ping.rand)?;

    calculate_checksum(&mut pck[..]);
    Ok(pck)
}

fn decode_pong_packet(pck: Vec<u8>) -> std::io::Result<(IcmpHeader, Option<IcmpPingPong>)> {
    let mut buf = &pck[..];

    let header = IcmpHeader {
        ty: buf.read_u8()?,
        code: buf.read_u8()?,
        checksum: buf.read_u16::<LittleEndian>()?,
    };

    let body = if header.ty == 0 {
        Some(IcmpPingPong {
            timestamp: buf.read_u128::<LittleEndian>()?,
            seq: buf.read_u64::<LittleEndian>()?,
            rand: buf.read_u64::<LittleEndian>()?,       
        })
    } else {
        None
    };
    Ok((
        header, body
    ))
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
    /// Token used to access your network (if this is omitted then the token path will be probed)
    #[clap(long)]
    pub token: Option<String>,
    /// Token file to read that holds a previously created token to be used for this operation
    #[cfg(not(target_os = "wasi"))]
    #[clap(long, default_value = "~/wasmer/token")]
    pub token_path: String,
    /// Token file to read that holds a previously created token to be used for this operation
    #[cfg(target_os = "wasi")]
    #[clap(long, default_value = "/.private/token")]
    pub token_path: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    main_async().await?;
    std::process::exit(0);
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    
    let start = Instant::now();
    let destination = opts.destination;
    let count = opts.count as u64;
    let interval = Duration::from_millis(opts.interval);
    let timeout = Duration::from_millis(opts.timeout);

    let mut dups = HashMap::<u64, u32>::new();
    let mut sent = 0u64;
    let mut received = 0u64;

    let port = Port::new(match opts.token {
            Some(token) => {
                let token: NetworkToken = std::str::FromStr::from_str(&token).unwrap();
                TokenSource::ByValue(token)
            },
            None => TokenSource::ByPath(opts.token_path)
        },
        url::Url::parse("wss://wasmer.sh/net").unwrap(),
        StreamSecurity::AnyEncryption)?;
    port.dhcp_acquire().await?;

    print!("connected:");
    if let Ok(Some(mac)) = port.hardware_address().await {
        print!(" mac={}", mac);
    }
    let ip = if let Ok(Some(ip)) = port.addr_ipv4().await {
        print!(" ip={}", ip);
        ip
    } else {
        panic!("no local ip address!");
    };
    println!("");

    println!("PING {} ({}) 56(84) bytes of data", destination, destination);

    let socket = port.bind_icmp(IpAddr::V4(ip)).await?;
    for seq in 0..count
    {
        let pck = create_ping_packet(start, seq)?;
        socket.send_to(pck, destination).await?;
        sent += 1;

        let wait = if sent < count {
            interval
        } else {
            timeout
        };

        let interval_start = Instant::now();
        while Instant::now() - interval_start < wait {
            tokio::select! {
                _ = sleep(Duration::from_millis(10)) => { },
                ret = socket.recv_from() => {
                    let (mut pck, from) = ret?;
                    let pck_len = pck.len();
                    let checksum_ok = check_checksum(&mut pck[..]);
                    match decode_pong_packet(pck) {
                        Ok((header, pong)) => {
                            match (header.ty, pong) {
                                (0, Some(pong)) => { //IcmpType::EchoReply
                                    let seq = pong.seq;
                                    let duration = Duration::from_nanos(pong.timestamp as u64);
                                    let duration = match Instant::now().duration_since(start).checked_sub(duration) {
                                        Some(a) => a,
                                        None => Duration::ZERO
                                    };
                                    print!("{} bytes from {}: icmp_seq={} ttl=64 time={}ms", pck_len, from, pong.seq, duration.as_millis());

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
                                }
                                (0, None) => { //IcmpType::EchoReply
                                    println!("{} bytes from {}: missing body", pck_len, from);
                                }
                                (3, _) => { //IcmpType::DestinationUnreachable
                                    let msg = match header.code {
                                        0 => "Destination Network Unreachable",
                                        1 => "Destination Host Unreachable",
                                        2 => "Protocol Unreachable",
                                        3 => "Port Unreachable",
                                        4 => "Fragmentation Needed",
                                        5 => "Source Route Failed",
                                        6 => "Destination Network Unknown",
                                        7 => "Destination Host Unknown",
                                        8 => "Source Host Isolated",
                                        9 => "Destination Network Prohibited",
                                        10 => "Destination Host Prohibited",
                                        _ => "Destination Unreachable"
                                    };
                                    println!("From {} icmp_seq={} {}", from, seq, msg);
                                }
                                _ => { }
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