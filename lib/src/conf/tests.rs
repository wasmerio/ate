#![cfg(test)]
#[allow(unused_imports)]
use log::{info, error, debug};
#[cfg(feature="enable_dns")]
use std::{net::IpAddr, str::FromStr};

use super::*;

pub(crate) fn mock_test_config() -> ConfAte {
    #[allow(unused_mut)]
    let mut ret = ConfAte::default();
    #[cfg(feature = "enable_local_fs")]
    {
        ret.log_path = Some("/tmp/ate".to_string());
    }
    return ret;
}

pub(crate) fn mock_test_mesh() -> ConfMesh {
    let mut ret = ConfMesh::for_domain("localhost".to_string());
    #[cfg(feature="enable_dns")]
    ret.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), 4001));
    #[cfg(not(feature="enable_dns"))]
    ret.roots.push(MeshAddress::new("localhost", 4001));
    ret
}

#[test]
fn test_config_mocking() {
    crate::utils::bootstrap_env();

    let cfg = mock_test_mesh();
    assert_eq!(cfg.roots.iter().next().unwrap().host.to_string(), "127.0.0.1");
}