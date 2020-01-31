extern crate tokio;
extern crate futures;
extern crate regex;
extern crate lazy_static;

pub(crate) mod util;
mod tcppm;
mod socks4;
mod socks5;
mod http;

#[tokio::main]
async fn main() {
    //tcppm(6666, "127.0.0.1:6667".parse().unwrap()).await;
    //socks5::socks5(6666).await;
    http::http(6666).await;
}