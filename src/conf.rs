pub trait ConfigMaster {
    fn master_addr(&self) -> String;
    fn master_port(&self) -> u32;
}

pub trait ConfigStorage {
    fn log_path(&self) -> String;
}
pub trait Config: ConfigMaster + ConfigStorage {
}

#[cfg(test)]
pub fn mock_test_config() -> impl Config {
    struct MockConfig {}

    impl ConfigMaster for MockConfig {
        fn master_addr(&self) -> String { "127.0.0.1".to_string() }
        fn master_port(&self) -> u32 { 4001 }
    }

    impl ConfigStorage for MockConfig {
        fn log_path(&self) -> String { "/tmp".to_string() }
    }

    impl Config for MockConfig {}

    MockConfig {}
}

#[test]
fn test_config_mocking() {
    let cfg = mock_test_config();

    assert_eq!(cfg.master_addr(), "127.0.0.1");
}