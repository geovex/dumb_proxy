use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use super::util;

#[allow(dead_code)]
pub async fn tcppm(src_port: u16, target: SocketAddr) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (src, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            if let Ok(dst) = TcpStream::connect(target).await {
                util::tcp_tranciever(src, dst).await.ok();
            }
        });
    }
}