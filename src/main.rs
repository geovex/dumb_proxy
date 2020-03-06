extern crate tokio;
extern crate tokio_io_timeout;
extern crate futures;
extern crate config;
extern crate serde;
extern crate serde_derive;
extern crate nom;
extern crate lru_cache;

pub(crate) mod util;
mod tcppm;
mod socks4;
mod socks5;
mod http;
mod config_loader;
mod config_spawner;
use std::env;

#[tokio::main]
async fn main() {
    let c = if env::args().len() > 1 {
        config_loader::load_config(env::args().nth(2).unwrap())
    } else {
        println!("using default configuration");
        config_loader::load_config_default()
    };
    config_spawner::spawn(c).await;
}