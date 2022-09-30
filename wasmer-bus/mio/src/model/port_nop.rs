use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortNopType
{
    MaySend,
    MayReceive,
    CloseHandle,
    BindRaw,
    BindIcmp,
    BindDhcp,
    DhcpReset,
    BindUdp,
    ConnectTcp,
    Listen,
    DhcpAcquire,
    SetHopLimit,
    SetAckDelay,
    SetNoDelay,
    SetPromiscuous,
    SetTimeout,
    SetKeepAlive,
}
