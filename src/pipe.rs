#[allow(unused_imports)]
use crate::meta::*;
use super::error::*;
use super::transaction::*;

pub trait EventPipe
{
    #[allow(dead_code)]
    fn feed(&self, trans: Transaction) -> Result<(), FeedError>;
}