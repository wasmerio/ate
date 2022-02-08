use serde::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendResult {
    Success(usize),
    Failed(String),
}