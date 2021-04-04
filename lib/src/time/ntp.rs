use std::fmt::Debug;
use std::fmt::Formatter;
use std::io;
use std::mem;
use std::net;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str;
use std::time;
use std::time::Duration;
use super::TimeError;

const MODE_MASK: u8 = 0b0000_0111;
const MODE_SHIFT: u8 = 0;
const VERSION_MASK: u8 = 0b0011_1000;
const VERSION_SHIFT: u8 = 3;
const LI_MASK: u8 = 0b1100_0000;
const LI_SHIFT: u8 = 6;
const NSEC_IN_SEC: u32 = 1_000_000_000;

struct NtpPacket {
    li_vn_mode: u8,
    stratum: u8,
    poll: i8,
    precision: i8,
    root_delay: u32,
    root_dispersion: u32,
    ref_id: u32,
    ref_timestamp: u64,
    origin_timestamp: u64,
    recv_timestamp: u64,
    tx_timestamp: u64,
}

/// SNTP request result representation
pub(crate) struct NtpResult {
    /// NTP server seconds value
    pub sec: u32,
    /// NTP server nanoseconds value
    pub nsec: u32,
    /// Request roundtrip time
    pub roundtrip: u64,
    /// Offset of the current system time with one received from a NTP server
    pub offset: i64,
}

impl NtpResult {
    /// Create new NTP result
    /// Args:
    /// * `sec` - number of seconds
    /// * `nsec` - number of nanoseconds
    /// * `roundtrip` - calculated roundtrip in microseconds
    /// * `offset` - calculated system clock offset in microseconds
    pub(crate) fn new(sec: u32, nsec: u32, roundtrip: u64, offset: i64) -> Self {
        let residue = nsec / NSEC_IN_SEC;
        let nsec = nsec % NSEC_IN_SEC;
        let sec = sec + residue;

        NtpResult {
            sec,
            nsec,
            roundtrip,
            offset,
        }
    }
    /// Returns number of seconds reported by an NTP server
    #[allow(dead_code)]
    pub(crate) fn sec(&self) -> u32 {
        self.sec
    }

    /// Returns number of nanoseconds reported by an NTP server
    #[allow(dead_code)]
    pub(crate) fn nsec(&self) -> u32 {
        self.nsec
    }

    /// Returns request's roundtrip time (client -> server -> client) in microseconds
    pub(crate) fn roundtrip(&self) -> u64 {
        self.roundtrip
    }

    /// Returns system clock offset value in microseconds
    pub(crate) fn offset(&self) -> i64 {
        self.offset
    }
}

impl Debug for NtpResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NtpResult")
            .field("sec", &self.sec)
            .field("nsec", &self.nsec)
            .field("roundtrip", &self.roundtrip)
            .field("offset", &self.offset)
            .finish()
    }
}

impl NtpPacket {
    const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;
    #[allow(dead_code)]
    const LI_MASK: u8 = 0b0000_0011;
    #[allow(dead_code)]
    const VN_MASK: u8 = 0b0001_1100;
    #[allow(dead_code)]
    const MODE_MASK: u8 = 0b1110_0000;

    pub(crate) fn new() -> NtpPacket {
        let tx_timestamp = get_ntp_timestamp();

        NtpPacket {
            li_vn_mode: NtpPacket::SNTP_CLIENT_MODE | NtpPacket::SNTP_VERSION,
            stratum: 0,
            poll: 0,
            precision: 0,
            root_delay: 0,
            root_dispersion: 0,
            ref_id: 0,
            ref_timestamp: 0,
            origin_timestamp: 0,
            recv_timestamp: 0,
            tx_timestamp,
        }
    }
}

trait NtpNum {
    type Type;

    fn ntohl(&self) -> Self::Type;
}

impl NtpNum for u32 {
    type Type = u32;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}
impl NtpNum for u64 {
    type Type = u64;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}

struct RawNtpPacket([u8; mem::size_of::<NtpPacket>()]);

impl Default for RawNtpPacket {
    fn default() -> Self {
        RawNtpPacket([0u8; mem::size_of::<NtpPacket>()])
    }
}

impl From<RawNtpPacket> for NtpPacket {
    fn from(val: RawNtpPacket) -> Self {
        // left it here for a while, maybe in future Rust releases there
        // will be a way to use such a generic function with compile-time
        // size determination
        // const fn to_array<T: Sized>(x: &[u8]) -> [u8; mem::size_of::<T>()] {
        //     let mut temp_buf = [0u8; mem::size_of::<T>()];
        //
        //     temp_buf.copy_from_slice(x);
        //     temp_buf
        // }
        let to_array_u32 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u32>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };
        let to_array_u64 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u64>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };

        NtpPacket {
            li_vn_mode: val.0[0],
            stratum: val.0[1],
            poll: val.0[2] as i8,
            precision: val.0[3] as i8,
            root_delay: u32::from_le_bytes(to_array_u32(&val.0[4..8])),
            root_dispersion: u32::from_le_bytes(to_array_u32(&val.0[8..12])),
            ref_id: u32::from_le_bytes(to_array_u32(&val.0[12..16])),
            ref_timestamp: u64::from_le_bytes(to_array_u64(&val.0[16..24])),
            origin_timestamp: u64::from_le_bytes(to_array_u64(&val.0[24..32])),
            recv_timestamp: u64::from_le_bytes(to_array_u64(&val.0[32..40])),
            tx_timestamp: u64::from_le_bytes(to_array_u64(&val.0[40..48])),
        }
    }
}

impl From<&NtpPacket> for RawNtpPacket {
    fn from(val: &NtpPacket) -> Self {
        let mut tmp_buf = [0u8; mem::size_of::<NtpPacket>()];

        tmp_buf[0] = val.li_vn_mode;
        tmp_buf[1] = val.stratum;
        tmp_buf[2] = val.poll as u8;
        tmp_buf[3] = val.precision as u8;
        tmp_buf[4..8].copy_from_slice(&val.root_delay.to_be_bytes());
        tmp_buf[8..12].copy_from_slice(&val.root_dispersion.to_be_bytes());
        tmp_buf[12..16].copy_from_slice(&val.ref_id.to_be_bytes());
        tmp_buf[16..24].copy_from_slice(&val.ref_timestamp.to_be_bytes());
        tmp_buf[24..32].copy_from_slice(&val.origin_timestamp.to_be_bytes());
        tmp_buf[32..40].copy_from_slice(&val.recv_timestamp.to_be_bytes());
        tmp_buf[40..48].copy_from_slice(&val.tx_timestamp.to_be_bytes());

        RawNtpPacket(tmp_buf)
    }
}

pub(crate) fn request(pool: &str, port: u32, timeout: time::Duration) -> io::Result<NtpResult> {
    let socket = net::UdpSocket::bind("0.0.0.0:0")
        .expect("Unable to create a UDP socket");
    let dest = format!("{}:{}", pool, port).to_socket_addrs()?;

    socket
        .set_read_timeout(Some(timeout))
        .expect("Unable to set up socket timeout");

    let req = NtpPacket::new();
    let dest = process_request(dest, &req, &socket)?;
    let mut buf: RawNtpPacket = RawNtpPacket::default();
    let (response, src) = socket.recv_from(buf.0.as_mut())?;
    let recv_timestamp = get_ntp_timestamp();
    
    if src != dest {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "SNTP response port / address mismatch",
        ));
    }

    if response == mem::size_of::<NtpPacket>() {
        let result = process_response(&req, buf, recv_timestamp);

        return match result {
            Ok(result) => {
                Ok(result)
            }
            Err(err_str) => Err(io::Error::new(io::ErrorKind::Other, err_str)),
        };
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Incorrect NTP packet size read",
    ))
}

fn process_request(
    dest: std::vec::IntoIter<SocketAddr>,
    req: &NtpPacket,
    socket: &UdpSocket,
) -> io::Result<SocketAddr> {
    for addr in dest {
        match send_request(&req, &socket, addr) {
            Ok(write_bytes) => {
                assert_eq!(write_bytes, mem::size_of::<NtpPacket>());
                return Ok(addr);
            }
            Err(_) => { },
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "SNTP servers not responding",
    ))
}

fn send_request(
    req: &NtpPacket,
    socket: &net::UdpSocket,
    dest: net::SocketAddr,
) -> io::Result<usize> {
    let buf: RawNtpPacket = req.into();

    socket.send_to(&buf.0, dest)
}

fn process_response(
    req: &NtpPacket,
    resp: RawNtpPacket,
    recv_timestamp: u64,
)
 -> Result<NtpResult, &str>
{
    const SNTP_UNICAST: u8 = 4;
    const SNTP_BROADCAST: u8 = 5;
    const LI_MAX_VALUE: u8 = 3;
    const MSEC_MASK: u64 = 0x0000_0000_ffff_ffff;
    let shifter = |val, mask, shift| (val & mask) >> shift;
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);
    
    if req.tx_timestamp != packet.origin_timestamp {
        return Err("Incorrect origin timestamp");
    }
    // Shift is 0
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let resp_version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let req_version = shifter(req.li_vn_mode, VERSION_MASK, VERSION_SHIFT);

    if mode != SNTP_UNICAST && mode != SNTP_BROADCAST {
        return Err("Incorrect MODE value");
    }

    if li > LI_MAX_VALUE {
        return Err("Incorrect LI value");
    }

    if req_version != resp_version {
        return Err("Incorrect response version");
    }

    if packet.stratum == 0 {
        return Err("Incorrect STRATUM headers");
    }
    //    theta = T(B) - T(A) = 1/2 * [(T2-T1) + (T3-T4)]
    //    and the round-trip delay
    //    delta = T(ABA) = (T4-T1) - (T3-T2).
    //    where:
    //      - T1 = client's TX timestamp
    //      - T2 = server's RX timestamp
    //      - T3 = server's TX timestamp
    //      - T4 = client's RX timestamp
    let delta = (recv_timestamp - packet.origin_timestamp) as i64
        - (packet.tx_timestamp - packet.recv_timestamp) as i64;
    let theta = ((packet.recv_timestamp as i64
        - packet.origin_timestamp as i64)
        + (recv_timestamp as i64 - packet.tx_timestamp as i64))
        / 2;

    let seconds = (packet.tx_timestamp >> 32) as u32;
    let nsec = (packet.tx_timestamp & MSEC_MASK) as u32;
    let tx_tm = seconds - NtpPacket::NTP_TIMESTAMP_DELTA;

    Ok(NtpResult::new(tx_tm, nsec, delta.abs() as u64, theta))
}

fn convert_from_network(packet: &mut NtpPacket) {
    fn ntohl<T: NtpNum>(val: T) -> T::Type {
        val.ntohl()
    }

    packet.root_delay = ntohl(packet.root_delay);
    packet.root_dispersion = ntohl(packet.root_dispersion);
    packet.ref_id = ntohl(packet.ref_id);
    packet.ref_timestamp = ntohl(packet.ref_timestamp);
    packet.origin_timestamp = ntohl(packet.origin_timestamp);
    packet.recv_timestamp = ntohl(packet.recv_timestamp);
    packet.tx_timestamp = ntohl(packet.tx_timestamp);
}

fn get_ntp_timestamp() -> u64 {
    let now_since_unix = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap();
    let timestamp = ((now_since_unix.as_secs()
        + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
        << 32)
        + u64::from(now_since_unix.subsec_micros());

    timestamp
}

pub(crate) fn query_ntp(pool: &String, port: u32, tolerance_ms: u32) -> Result<NtpResult, TimeError>
{
    let timeout =  Duration::from_millis(tolerance_ms as u64) + Duration::from_millis(50);
    let ret = request(pool.as_str(), port, timeout)?;
    let ping = Duration::from_micros(ret.roundtrip()).as_millis() as u32;
    if ping > tolerance_ms {
        return Err(TimeError::BeyondTolerance(ping as u32));
    }
    Ok(ret)
}

pub(crate) async fn query_ntp_retry(pool: &String, port: u32, tolerance_ms: u32, samples: u32) -> Result<NtpResult, TimeError>
{
    let mut best: Option<NtpResult> = None;
    let mut positives = 0;
    let mut wait_time = 50;

    for _ in 0..samples
    {
        let timeout = match &best {
            Some(b) => Duration::from_micros(b.roundtrip()) + Duration::from_millis(50),
            None => Duration::from_millis(tolerance_ms as u64),
        };

        if let Ok(ret) = request(pool.as_str(), port, timeout) {
            let current_ping = match &best {
                Some(b) => b.roundtrip(),
                None => u64::max_value(),
            };
            if ret.roundtrip() < current_ping {
                best = Some(ret);
            }
            positives = positives + 1;
            if positives >= samples {
                break;
            }
        }
        else
        {
            tokio::time::sleep(Duration::from_millis(wait_time)).await;
            wait_time = (wait_time * 120) / 100;
             wait_time = wait_time + 50;
        }
    }

    if let Some(ret) = best {
        let ping = Duration::from_micros(ret.roundtrip()).as_millis() as u32;
        if ping <= tolerance_ms {
            return Ok(ret);
        }
    }

    query_ntp(pool, port, tolerance_ms)
}