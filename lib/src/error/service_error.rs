use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceErrorReply<E>
{
    Reply(E),
    ServiceError(String),
}