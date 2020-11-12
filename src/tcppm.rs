use super::util;
use crate::logger;
use tokio::net::{TcpListener, TcpStream};

pub async fn tcppm(name: String, src_port: u16, target: String) {
    let listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    dbg!(&listener);
    loop {
        let (mut src, _addr) = listener.accept().await.unwrap();
        let name_clone = name.clone();
        let target_clone = target.clone();
        tokio::spawn(async move {
            if let Ok(dst_addr) = util::resolve_sockaddr(&target_clone).await {
                dbg!(&dst_addr);
                if let Ok(mut dst) = TcpStream::connect(dst_addr).await {
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
                        "tcppm.{} failed to connect to {} {:?}",
                        name_clone, target_clone, dst_addr
                    ));
                }
            } else {
                logger::log(format!(
                    "tcppm.{} resolve {} failed",
                    name_clone, target_clone
                ));
            };
        });
    }
}
