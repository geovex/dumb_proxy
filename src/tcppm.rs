use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use super::util;

pub async fn tcppm(src_port: u16, target: SocketAddr) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (mut src, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            if let Ok(mut dst) = TcpStream::connect(target).await {
                src.set_nodelay(true).ok();
                dst.set_nodelay(true).ok();
                util::transceiver(&mut src, &mut dst).await.ok();
            }
        });
    }
}