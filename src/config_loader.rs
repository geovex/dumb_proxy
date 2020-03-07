use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use toml;

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
    let mut buffer = String::new();
    File::open(path.as_ref()).unwrap().read_to_string(&mut buffer).unwrap();
    toml::from_str(&buffer).unwrap()
}

pub fn load_config_default() -> Config {
    let t = toml::toml! {
        [http.a]
        port = 3128
    };
    t.try_into().unwrap()
}
