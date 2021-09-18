mod web_conf;

pub use web_conf::*;

pub const WEB_CONF_FILES: &'static str = ".conf/";
pub const WEB_CONF_FILES_CERT: &'static str = ".conf/cert.pem";
pub const WEB_CONF_FILES_KEY: &'static str = ".conf/key.pem";
pub const WEB_CONF_FILES_CONF: &'static str = ".conf/web.yaml";