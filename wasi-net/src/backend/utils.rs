#![allow(dead_code)]
use std::io;
use std::io::ErrorKind;
use std::io::Read;

use serde::Deserialize;
use serde::de;

pub fn read_line(file: &mut std::fs::File) -> String {
    let mut line = String::new();
    loop {
        let mut buf = [0u8; 4096];
        match file.read(&mut buf[..]) {
            Ok(read) if read == 0usize => {
                break;
            }
            Ok(read) => {
                let a = String::from_utf8_lossy(&buf[..read]);
                if line.ends_with("\n") {
                    let end = line.len() - 1;
                    line += &a[..end];
                    break;
                }
                line += a.as_ref();
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                std::thread::yield_now();
                continue;
            }
            Err(_) => {
                break;
            }
        }
    }
    return line;
}

pub fn read_to_end(file: &mut std::fs::File, data: &mut Vec<u8>) -> Result<(), std::io::Error> {
    let mut buf = [0u8; 4096];
    loop {
        match file.read(&mut buf[..]) {
            Ok(read) if read == 0usize => {
                break;
            }
            Ok(read) => {
                data.extend_from_slice(&buf[..read]);
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                std::thread::yield_now();
                continue;
            }
            Err(err)
                if err.kind() == ErrorKind::ConnectionAborted
                    || err.kind() == ErrorKind::ConnectionReset
                    || err.kind() == ErrorKind::BrokenPipe =>
            {
                break;
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    return Ok(());
}

pub fn read_response<T>(file: &mut std::fs::File) -> io::Result<T>
where T: de::DeserializeOwned
{
    let res = read_line(file);
    let res = base64::decode(res.trim()).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("failed to base64 decode the response - {}", err).as_str(),
        )
    })?;
    let res: T = bincode::deserialize(&res[..]).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("failed to deserialize the response - {}", err).as_str(),
        )
    })?;

    Ok(res)
}