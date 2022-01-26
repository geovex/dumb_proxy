use super::util;
use crate::logger;
use tokio::net::{TcpListener, TcpStream};

pub async fn tcppm(name: String, src_port: u16, target: String) {
    let listener = util::bind_listener(src_port).await;
    loop {
        let (mut src, _addr) = listener.accept().await.unwrap();
        let name_clone = name.clone();
        let target_clone = target.clone();
        tokio::spawn(async move {
            if let Ok(mut dst) = TcpStream::connect(&target_clone).await {
                src.set_nodelay(true).ok();
                dst.set_nodelay(true).ok();
                logger::log(format!(
                    "tcppm.{} {:?} -> {:?}",
                    name_clone,
                    src.peer_addr().unwrap(),
                    dst.peer_addr().unwrap()
                ));
                util::transceiver(&mut src, &mut dst).await.ok();
            } else {
                logger::log(format!(
                    "tcppm.{} failed to connect to {}",
                    name_clone, target_clone
                ));
            }
        });
    }
}
