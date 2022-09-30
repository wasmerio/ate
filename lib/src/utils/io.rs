use std::fmt;
use std::fs::File;
use std::str::FromStr;
use std::io::{BufReader, BufRead, Error, Read};

pub fn load_node_list(list: Option<String>) -> Option<Vec<String>>
{
    match list {
        Some(list) => {
            let list = shellexpand::tilde(&list).to_string();
            let file = File::open(list.as_str())
                .map_err(|err| conv_file_open_err(list.as_str(), err))
                .unwrap();
            let reader = BufReader::new(file);

            let mut ret = Vec::new();
            for line in reader.lines() {
                ret.push(line.unwrap());
            }
            Some(ret)
        },
        None => None
    }
}

pub fn load_node_id(path: Option<String>) -> Option<u32>
{
    match path {
        Some(path) => {
            let path = shellexpand::tilde(&path).to_string();
            let mut file = File::open(path.as_str())
                .map_err(|err| conv_file_open_err(path.as_str(), err))
                .unwrap();
            let mut ret = String::new();
            if let Ok(_) = file.read_to_string(&mut ret) {
                u32::from_str(ret.as_str()).ok()
            } else {
                None
            }
        },
        None => None
    }
}

pub fn conv_file_open_err(path: &str, inner: Error) -> Error {
    Error::new(inner.kind(), FileIOError {
        path: path.to_string(),
        inner
    })
}

pub struct FileIOError
{
    path: String,
    inner: Error,
}

impl fmt::Display
for FileIOError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed while attempting to access [{}] - {}", self.path, self.inner.to_string())
    }
}

impl fmt::Debug
for FileIOError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error
for FileIOError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}
