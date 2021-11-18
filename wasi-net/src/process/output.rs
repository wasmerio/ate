use super::*;

#[derive(PartialEq, Eq, Clone)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}