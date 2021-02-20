pub trait ConfigMaster {
    fn master_addr(&self) -> String;
    fn master_port(&self) -> u32;
}

pub trait ConfigStorage {
    fn log_path(&self) -> String;
    fn log_temp(&self) -> bool;
}
pub trait Config: ConfigMaster + ConfigStorage {
}

#[derive(Default)]
pub struct DiscreteConfig {
    pub master_addr: String,
    pub master_port: u32,

    pub log_path: String,
    pub log_temp: bool,
}

impl DiscreteConfig {
    #[allow(dead_code)]
    pub fn with_master_addr(mut self, val: String) -> DiscreteConfig {
        self.master_addr = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_master_port(mut self, val: u32) -> DiscreteConfig {
        self.master_port = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_log_path(mut self, val: String) -> DiscreteConfig {
        self.log_path = val;
        self
    }

    #[allow(dead_code)]
    pub fn with_log_temp(mut self, val: bool) -> DiscreteConfig {
        self.log_temp = val;
        self
    }
}

impl ConfigMaster for DiscreteConfig {
    fn master_addr(&self) -> String { self.master_addr.clone() }
    fn master_port(&self) -> u32 { self.master_port }
}


impl ConfigStorage for DiscreteConfig {
    fn log_path(&self) -> String { self.log_path.clone() }
    fn log_temp(&self) -> bool { self.log_temp }
}


impl Config for DiscreteConfig {
}

#[cfg(test)]
pub fn mock_test_config() -> DiscreteConfig {
    DiscreteConfig::default()
        .with_master_addr("127.0.0.1".to_string())
        .with_master_port(4001)
        .with_log_path("/tmp/ate".to_string())
        .with_log_temp(true)
}

#[test]
fn test_config_mocking() {
    let cfg = mock_test_config();
    assert_eq!(cfg.master_addr(), "127.0.0.1");
}