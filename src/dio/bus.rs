use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

use super::Dao;
use crate::error::*;

#[allow(dead_code)]
struct Bus<D>
where D: Serialize + DeserializeOwned + Clone
{
    _marker: PhantomData<D>,
}

impl<D> Bus<D>
where D: Serialize + DeserializeOwned + Clone,
{
    #[allow(dead_code)]
    pub async fn recv(&self) -> Result<Dao<D>, BusError> {
        Err(BusError::NotImplemented)
    }

    #[allow(dead_code)]
    pub async fn send(&self, _data: D) -> Result<(), BusError> {
        Err(BusError::NotImplemented)
    }
}