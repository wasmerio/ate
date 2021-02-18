extern crate tokio;

use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use std::{fs, mem};

#[allow(unused_imports)]
use super::Config;

pub struct SplitLogFile {
    pub offsets: std::fs::File,
    pub headers: std::fs::File,
    pub data: std::fs::File,
}

impl SplitLogFile {
    fn new() -> SplitLogFile {
        SplitLogFile {
            offsets: std::fs::File::create("/tmp/log.offsets").unwrap(),
            headers: std::fs::File::create("/tmp/log.headers").unwrap(),
            data: std::fs::File::create("/tmp/log.data").unwrap(),
        }
    }
}
#[allow(dead_code)]
pub enum LoggingMode {
    FrontOnly,
    BothBuffers,
    Blocking
}

pub struct RedoLog {
    pub front: SplitLogFile,
    pub back: SplitLogFile,
    pub mode: LoggingMode,
}

impl RedoLog
{
    #[allow(dead_code)]
    pub fn new() -> RedoLog {
        RedoLog {
            front: SplitLogFile::new(),
            back: SplitLogFile::new(),
            mode: LoggingMode::FrontOnly,
        }
    }

    #[allow(dead_code)]
    pub fn write<T: Serialize>(&self, obj: T) {
        let _bytes = bincode::serialize(&obj).unwrap();
    }

    #[allow(dead_code)]
    pub fn read<'a, T: Deserialize<'a>>(&self, bytes: &'a Vec<u8>) -> T {
        bincode::deserialize(&bytes[..]).unwrap()
    }
}

#[test]
fn test_redo_log() {
    let _rl = RedoLog::new();
    //panic!("test!")
}