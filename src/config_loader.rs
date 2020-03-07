use config;
use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub http: HashMap<String, HttpConfig>,
    pub socks4: HashMap<String, Socks4Config>,
    pub socks5: HashMap<String, Socks5Config>,
    pub tcppm: HashMap<String, TcpPmConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HttpConfig {
    pub port: u16
}

#[derive(Deserialize, Debug, Clone)]
pub struct Socks4Config {
    pub port: u16
}

#[derive(Deserialize, Debug, Clone)]
pub struct Socks5Config {
    pub port: u16
}

#[derive(Deserialize, Debug, Clone)]
pub struct TcpPmConfig {
    pub port: u16,
    pub target: String
}

pub fn load_config<P: AsRef<str>>(path: P) -> Config {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::new(path.as_ref(), config::FileFormat::Toml))
        .unwrap();
    settings.try_into().unwrap()
}

const DEFAULT_CONFIG: &str = "
[http.a]
port = 3128
";

pub fn load_config_default() -> Config {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::from_str(
            DEFAULT_CONFIG,
            config::FileFormat::Toml,
        ))
        .unwrap();
    settings.try_into().unwrap()
}
