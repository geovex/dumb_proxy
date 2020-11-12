use super::util;
use crate::logger;
use tokio::net::{TcpListener, TcpStream};

use nom::{Err, IResult, Needed};
use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt};

mod parser;

enum Socks5Error {
    Handshake,
    InvalidAuth,
    InvalidRequest,
    TargetUnreachable,
    Transceiver,
}

type Socks5Result<T> = Result<T, Socks5Error>;

#[derive(Debug)]
pub struct AuthRequest {
    //VER
    //NAUTH
    pub auths: Vec<u8>,
}

pub enum RequestAddr {
    Ip(IpAddr),
    Domain(String),
}
pub struct ConnectRequest {
    //VER
    pub cmd: u8,
    //RSV
    pub addr: RequestAddr,
    pub port: u16,
}

async fn parser_read<R, T, P>(stream: &mut R, parser: P) -> Option<T>
where
    R: AsyncRead + Unpin,
    P: Fn(&[u8]) -> IResult<&[u8], T>,
{
    let mut temp = Vec::new();
    loop {
        match parser(&temp) {
            Ok((_, result)) => return Some(result),
            Err(Err::Incomplete(Needed::Size(size))) => {
                let old_len = temp.len();
                temp.resize(old_len + size.get(), 0);
                stream.read_exact(&mut temp[old_len..]).await.ok()?;
            }
            Err(Err::Incomplete(Needed::Unknown)) => {
                let old_len = temp.len();
                temp.resize(old_len + 1, 0);
                stream.read_exact(&mut temp[old_len..]).await.ok()?;
            }
            _ => return None,
        }
    }
}

async fn socks5_parser(name: String, mut sock: TcpStream) -> Socks5Result<()> {
    use tokio::io::AsyncWriteExt;
    sock.set_nodelay(true).ok();
    let auth_requeest = parser_read(&mut sock, parser::parse_auth)
        .await
        .ok_or(Socks5Error::Handshake)?;
    if !auth_requeest.auths.contains(&0u8) {
        //only support no auth
        sock.write_all(&[0x5, 0xff]).await.ok();
        return Err(Socks5Error::InvalidAuth);
    }
    //successful auth
    sock.write_all(&[0x5, 0x0])
        .await
        .or(Err(Socks5Error::Handshake))?;
    let request = parser_read(&mut sock, parser::parse_request)
        .await
        .ok_or(Socks5Error::InvalidRequest)?;
    sock.write_all(&[0x5, 0x0, 0x0])
        .await
        .or(Err(Socks5Error::Handshake))?;
    let sockaddr = match request.addr {
        RequestAddr::Ip(addr) => SocketAddr::new(addr, request.port),
        RequestAddr::Domain(domain) => {
            let domain = format!("{}:{}", domain, request.port);
            util::resolve_sockaddr(domain)
                .await
                .or(Err(Socks5Error::TargetUnreachable))?
        }
    };
    let mut dest = TcpStream::connect(sockaddr)
        .await
        .or(Err(Socks5Error::TargetUnreachable))?;
    let reply_addr = match sockaddr {
        SocketAddr::V4(a) => {
            let mut result = vec![1];
            result.extend_from_slice(&a.ip().octets());
            result.push((a.port() >> 8) as u8);
            result.push(a.port() as u8);
            result
        }
        SocketAddr::V6(a) => {
            let mut result = vec![4];
            result.extend_from_slice(&a.ip().octets());
            result.push((a.port() >> 8) as u8);
            result.push(a.port() as u8);
            result
        }
    };
    sock.write_all(&reply_addr)
        .await
        .or(Err(Socks5Error::Handshake))?;
    logger::log(format!(
        "socks5.{} {:?} -> {:?}",
        name,
        sock.peer_addr().or(Err(Socks5Error::Handshake))?,
        dest.peer_addr().or(Err(Socks5Error::Handshake))?
    ));
    util::transceiver(&mut sock, &mut dest)
        .await
        .or(Err(Socks5Error::Transceiver))?;
    Ok(())
}

pub async fn socks5(name: String, src_port: u16) {
    let listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (sock, _addr) = listener.accept().await.unwrap();
        let name_clone = name.clone();
        tokio::spawn(async move { socks5_parser(name_clone, sock).await.ok() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn uniparser() {
        let data = [5u8, 2, 0, 1];
        let auth_req = parser_read(&mut &data[..], parser::parse_auth)
            .await
            .unwrap();
        assert_eq!(auth_req.auths, [0, 1]);
    }
}
