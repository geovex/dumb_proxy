extern crate tokio;
extern crate futures;
extern crate config;
extern crate serde;
extern crate serde_derive;
extern crate nom;

pub(crate) mod util;
mod tcppm;
mod socks4;
mod socks5;
mod http;
mod config_loader;
mod config_spawner;

#[tokio::main]
async fn main() {
    let c = config_loader::load_config("config/default.toml");
    config_spawner::spawn(c).await;
}