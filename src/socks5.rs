use tokio::io::Result;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use super::util;

use std::net::{SocketAddr, IpAddr};

async fn socks5_parser(mut sock: TcpStream) -> Result<()> {
    async fn check_version(sock: &mut TcpStream) -> Result<()> {
        if 5 != sock.read_u8().await? {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid version"));
        } else {
            Ok(())
        }
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    //check version
    check_version(&mut sock).await?;
    //check auth version
    let nauth = sock.read_u8().await?;
    let mut auth = vec![0u8; nauth as usize];
    sock.read_exact(auth.as_mut_slice()).await?;
    if !auth.contains(&0u8) {
         //only support no auth
        sock.write_all(&[0x5, 0xff]).await.ok();
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid auth"));
    }
    dbg!(auth);
    //successful auth
    sock.write_all(&[0x5, 0x0]).await?;
    //client request
    check_version(&mut sock).await?;
    if 0x01 != sock.read_u8().await? { //command
        let reply = [
            0x5u8, //VER
            0x07, //invalid command
            0, //RSV reserved
            0x1, 0x0, 0x0, 0x0, 0x0, //zeroed ipv4
            0x0, 0x0]; //seroed port
        sock.write_all(&reply).await.ok();
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid command"));
    }
    sock.read_u8().await?; //reserved
    let atyp = sock.read_u8().await?; //address type
    let ipaddr = match atyp {
        1 => { //ipv4
            let mut addr = [0u8; 4];
            sock.read_exact(&mut addr).await?;
            IpAddr::from(addr)
        },
        3 => {
            use std::net::ToSocketAddrs;
            let len = sock.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            sock.read_exact(domain.as_mut_slice()).await?;
            let domain = std::str::from_utf8(domain.as_slice()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "domain parse error"))?;
            dbg!(&domain);
            let domain: String = domain.chars().filter(|x| *x != ':').collect(); //possible : injection
            let domain = format!("{}:10", domain); 
            util::resolve_sockaddr(domain).await?.ip()
        },
        4 => {//ipv6
            let mut addr = [0u8; 16];
            sock.read_exact(&mut addr).await?;
            IpAddr::from(addr)
        }
        _ => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid request"));
        }
    };
    dbg!(&ipaddr);
    let port = sock.read_u16().await?;
    let dest = TcpStream::connect(&SocketAddr::new(ipaddr, port)).await?;
    let dst_local_addr = dest.local_addr()?;
    sock.write_all(&[0x5, 0x0, 0x0]).await?;
    let reply_addr = match dst_local_addr {
        SocketAddr::V4(a) => { 
            let mut result = vec![1];
            result.extend_from_slice(&a.ip().octets());
            result.push((a.port() >> 8) as u8);
            result.push(a.port() as u8);
            result
        },
        SocketAddr::V6(a) => { 
            let mut result = vec![4];
            result.extend_from_slice(&a.ip().octets());
            result.push((a.port() >> 8) as u8);
            result.push(a.port() as u8);
            result
        },
    };
    sock.write_all(&reply_addr).await?;
    util::tcp_tranciever(sock, dest).await?;
    Ok(())
}

pub async fn socks5(src_port: u16) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (sock, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {socks5_parser(sock).await.ok()});
    }
}