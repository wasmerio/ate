#![cfg(test)]
#[cfg(feature = "enable_dns")]
use std::{net::IpAddr, str::FromStr};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::mesh::Registry;

use super::*;

pub(crate) fn mock_test_config() -> ConfAte {
    #[allow(unused_mut)]
    let mut ret = ConfAte::default();
    #[cfg(feature = "enable_local_fs")]
    {
        ret.log_path = Some("/tmp/ate".to_string());
    }
    #[cfg(feature = "enable_ntp")]
    {
        ret.ntp_sync = false;
    }
    return ret;
}

pub(crate) fn mock_test_mesh(port: u16) -> ConfMesh {
    let mut roots = Vec::new();
    #[cfg(feature = "enable_dns")]
    roots.push(MeshAddress::new(
        IpAddr::from_str("127.0.0.1").unwrap(),
        port,
    ));
    #[cfg(not(feature = "enable_dns"))]
    roots.push(MeshAddress::new("localhost", port));

    let remote = url::Url::parse(format!("{}://localhost", Registry::guess_schema(port)).as_str()).unwrap();
    let ret = ConfMesh::new("localhost", remote, roots.iter());
    ret
}

#[test]
fn test_config_mocking() {
    crate::utils::bootstrap_test_env();

    let cfg = mock_test_mesh(4001);
    assert_eq!(
        cfg.roots.iter().next().unwrap().host.to_string(),
        "127.0.0.1"
    );
}
