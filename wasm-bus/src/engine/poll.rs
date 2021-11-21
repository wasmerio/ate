use std::task::Context;

use crate::abi::*;
use super::*;

pub(crate) fn poll(handle: &CallHandle, cx: Option<&mut Context<'_>>) -> Option<Data>
{
    engine::BUS_ENGINE.get(handle, cx)
}

pub(crate) fn finish(handle: CallHandle, response: Data)
{
    engine::BUS_ENGINE.put(handle, response);
}

pub(crate) fn begin() -> CallHandle
{
    engine::BUS_ENGINE.generate()
}