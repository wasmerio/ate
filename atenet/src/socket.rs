use std::fmt;

pub trait Socket
where Self: fmt::Debug
{

}

#[derive(Debug)]
pub struct UdpSocket {

}

impl Socket
for UdpSocket {

}

#[derive(Debug)]
pub struct TcpSocket {

}

impl Socket
for TcpSocket {

}