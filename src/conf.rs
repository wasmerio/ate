#[derive(Debug, Clone)]
pub struct Config {
    pub master_addr: String,
    pub port: u32,
}

impl Config
{
    #[allow(dead_code)]
    pub fn new(addr: &str, port: u32) -> Config {
        Config {
            master_addr: String::from(addr),
            port: port,
        }
    }
}