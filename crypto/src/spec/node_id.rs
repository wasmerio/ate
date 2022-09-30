use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    Unknown,
    Client(u64),
    Server(u32, u32),
}

impl Default for NodeId {
    fn default() -> Self {
        NodeId::Unknown
    }
}

impl fmt::Display
for NodeId
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeId::Unknown => write!(f, "unknown"),
            NodeId::Client(id) => write!(f, "client({})", id),
            NodeId::Server(server_id, node_id) => write!(f, "server(id={}, node={})", server_id, node_id),
        }
    }
}

impl NodeId {
    pub fn generate_client_id() -> NodeId {
        let client_id = fastrand::u64(..);
        NodeId::Client(client_id)
    }

    pub fn generate_server_id(node_id: u32) -> NodeId {
        let server_id = fastrand::u32(..);
        NodeId::Server(server_id, node_id)
    }

    pub fn to_string(&self) -> String {
        match self {
            NodeId::Unknown => "[new]".to_string(),
            NodeId::Client(a) => hex::encode(a.to_be_bytes()).to_uppercase(),
            NodeId::Server(_, b) => format!("n{}", b),
        }
    }

    pub fn to_short_string(&self) -> String {
        match self {
            NodeId::Unknown => "[new]".to_string(),
            NodeId::Client(a) => {
                let client_id = hex::encode(a.to_be_bytes()).to_uppercase();
                format!("{}", &client_id[..4])
            }
            NodeId::Server(_, a) => format!("n{}", a),
        }
    }
}
