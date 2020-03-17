use super::util;
use crate::logger;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub async fn tcppm(name: String, src_port: u16, target: SocketAddr) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (mut src, _addr) = listener.accept().await.unwrap();
        let name_clone = name.clone();
        tokio::spawn(async move {
            if let Ok(mut dst) = TcpStream::connect(target).await {
                src.set_nodelay(true).ok();
                dst.set_nodelay(true).ok();
                logger::log(format!(
                    "tcppm.{} {:?} -> {:?}",
                    name_clone,
                    src.peer_addr().unwrap(),
                    dst.peer_addr().unwrap()
                ));
                util::transceiver(&mut src, &mut dst).await.ok();
            }
        });
    }
}
