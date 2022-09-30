use ate::comms::{StreamRx, Upstream};

/// Relays a web socket connection over to another server that
/// is currently hosting this particular instance
pub struct Relay
{
    pub rx: StreamRx,
    pub tx: Upstream,
}

impl Relay
{
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>>
    {
        Ok(())
    }
}