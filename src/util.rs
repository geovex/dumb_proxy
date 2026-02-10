use std::net::SocketAddr;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Result};
use tokio::net::TcpListener;

pub async fn transceiver<S, D>(src: &mut S, dst: &mut D) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
    D: AsyncRead + AsyncWrite + Unpin,
{
    let mut src_buf = [0u8; 2000];
    let mut dst_buf = [0u8; 2000];
    loop {
        tokio::select! {
            Ok(size) = src.read(&mut src_buf) => {
                if size == 0 {return Ok(())};
                dst.write_all(&src_buf[..size]).await?;
            }
            Ok(size) = dst.read(&mut dst_buf) => {
                if size == 0 {return Ok(())};
                src.write_all(&dst_buf[..size]).await?;
            }
            else => {
                return Ok(());
            }
        }
    }
}

pub async fn bind_listener(port: u16) -> TcpListener {
    let addr: SocketAddr = format!("[::]:{}", port).parse().unwrap();
    let std_listener = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)).unwrap();
    std_listener.set_only_v6(false).unwrap();
    std_listener.set_tcp_nodelay(true).unwrap();
    std_listener.set_reuse_address(true).unwrap();
    std_listener.set_nonblocking(true).unwrap();
    std_listener.bind(&addr.into()).unwrap();
    std_listener.listen(1024).unwrap();
    tokio::net::TcpListener::from_std(std_listener.into()).unwrap()
}

#[cfg(test)]
mod test {
    #[test]
    #[should_panic]
    fn invalid_resolve() {
        use std::net::ToSocketAddrs as _;
        "127.0.0.1:80:70".to_socket_addrs().unwrap();
    }
}
