extern crate tokio;
extern crate futures;

pub(crate) mod util;
mod tcppm;
mod socks4;

#[tokio::main]
async fn main() {
    //tcppm(6666, "127.0.0.1:6667".parse().unwrap()).await;
    socks4::socks4(6666).await;
}