#[allow(unused_imports)]
use log::{info, error, debug};

use super::*;

pub fn eat<T>(ret: Result<T, AteError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_load<T>(ret: Result<T, LoadError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_serialization<T>(ret: Result<T, SerializationError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_commit<T>(ret: Result<T, CommitError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}

pub fn eat_lock<T>(ret: Result<T, LockError>) -> Option<T> {
    match ret {
        Ok(a) => Some(a),
        Err(err) => {
            debug!("error: {}", err);
            None
        }
    }
}