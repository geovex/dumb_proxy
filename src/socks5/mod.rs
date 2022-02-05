use super::util;
use crate::config_loader::Socks5Config;
use crate::logger;
use tokio::net::TcpStream;

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

pub struct Socks5 {
    name: String,
    config: Socks5Config,
}

impl Socks5 {
    pub fn new(name: &String, config: &Socks5Config) -> Socks5 {
        Socks5 {
            name: name.clone(),
            config: config.clone(),
        }
    }
    pub async fn serve(&self) {
        let listener = util::bind_listener(self.config.port).await;
        loop {
            let (sock, _addr) = listener.accept().await.unwrap();
            let name_clone = self.name.clone();
            tokio::spawn(async move { Self::socks5_parser(name_clone, sock).await.ok() });
        }
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
        let auth_requeest = Self::parser_read(&mut sock, parser::parse_auth)
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
        let request = Self::parser_read(&mut sock, parser::parse_request)
            .await
            .ok_or(Socks5Error::InvalidRequest)?;
        sock.write_all(&[0x5, 0x0, 0x0])
            .await
            .or(Err(Socks5Error::Handshake))?;
        let mut dest = match request.addr {
            RequestAddr::Ip(addr) => TcpStream::connect(SocketAddr::new(addr, request.port))
                .await
                .or(Err(Socks5Error::TargetUnreachable))?,
            RequestAddr::Domain(domain) => {
                let domain = format!("{}:{}", domain, request.port);
                TcpStream::connect(domain)
                    .await
                    .or(Err(Socks5Error::TargetUnreachable))?
            }
        };
        let reply_addr = match dest.peer_addr().or(Err(Socks5Error::InvalidRequest))? {
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
}

#[derive(Debug)]
pub struct AuthRequest {
    //VER
    //NAUTH
    pub auths: Vec<u8>,
}

enum RequestAddr {
    Ip(IpAddr),
    Domain(String),
}
struct ConnectRequest {
    //VER
    _cmd: u8,
    //RSV
    addr: RequestAddr,
    port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn uniparser() {
        let data = [5u8, 2, 0, 1];
        let auth_req = Socks5::parser_read(&mut &data[..], parser::parse_auth)
            .await
            .unwrap();
        assert_eq!(auth_req.auths, [0, 1]);
    }
}
