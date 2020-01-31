use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};
use tokio::io;
use tokio::net::TcpStream;
use tokio;
use std::net::{ToSocketAddrs, SocketAddr};

pub async fn tcp_tranciever(mut src: TcpStream, mut dst: TcpStream) -> Result<()> {
    println!("tranciever");
    src.set_nodelay(true)?;
    dst.set_nodelay(true)?;
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

pub async fn resolve_sockaddr<S: Into<String>>(addr_port: S) -> Result<SocketAddr> {
    let string_addr_port = addr_port.into();
    let addrs = tokio::task::spawn_blocking(move || {
        dbg!(&string_addr_port);
        string_addr_port.to_socket_addrs()
    }).await?;
    match addrs {
        Ok(mut addr_list) => {Ok(addr_list.next().unwrap())},
        Err(err) => {
            dbg!(err);
            return Err(io::Error::new(io::ErrorKind::NotFound, "domain not found")
        )}
    }
}