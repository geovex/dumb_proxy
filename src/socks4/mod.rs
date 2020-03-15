use super::util;
use crate::logger;
use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod parser;

#[derive(Debug)]
pub enum Socks4Error {
    Handshake,
    HeaderInvalid,
    TargetUnreachable,
    Transceiver,
}

type Socks4Result<T> = Result<T, Socks4Error>;

const MAX_ID_LENGTH: usize = 1000;
pub struct Request {
    // VER 0x04
    pub cmd: u8,
    pub dst: SocketAddr,
    pub id: String,
}

async fn read_request<R>(sock: &mut R) -> Socks4Result<Request>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0u8; 8];
    sock.read_exact(&mut buf)
        .await
        .or(Err(Socks4Error::HeaderInvalid))?;
    let mut id = Vec::with_capacity(10);
    while id.ends_with(&[0]) && id.len() < MAX_ID_LENGTH {
        id.push(sock.read_u8().await.or(Err(Socks4Error::HeaderInvalid))?);
    }
    id.pop();
    let (_rest, (cmd, dstport, dstip)) =
        parser::pre_parser(&buf).or(Err(Socks4Error::HeaderInvalid))?;
    Ok(Request {
        cmd,
        dst: SocketAddr::new(IpAddr::V4(dstip), dstport),
        id: String::from_utf8_lossy(&id).into_owned(),
    })
}

async fn socks4_parser(mut sock: TcpStream) -> Socks4Result<()> {
    const GOOD_REPLY: [u8; 8] = [
        0x00u8, //VN
        0x5a,   //Granted
        0x00, 0x00, //DSTPORT,
        0x00, 0x00, 0x00, 0x00,
    ]; //DSTIP
    const BAD_REPLY: [u8; 8] = [
        0x00u8, //VN
        0x5b,   //Failed
        0x00, 0x00, //DSTPORT,
        0x00, 0x00, 0x00, 0x00,
    ]; //DSTIP
    sock.set_nodelay(true).ok();
    let request = read_request(&mut sock).await?;
    let dst = TcpStream::connect(&request.dst).await;
    if let Ok(mut dst) = dst {
        sock.write_all(&GOOD_REPLY)
            .await
            .or(Err(Socks4Error::Handshake))?;
        logger::log(format!(
            "socs4 {:?} {} -> {:?}",
            sock.peer_addr().or(Err(Socks4Error::Handshake))?,
            request.id,
            dst.peer_addr().or(Err(Socks4Error::Handshake))?
        ));
        util::transceiver(&mut sock, &mut dst)
            .await
            .or(Err(Socks4Error::Transceiver))
    } else {
        sock.write_all(&BAD_REPLY).await.ok();
        Err(Socks4Error::TargetUnreachable)
    }
}

pub async fn socks4(src_port: u16) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (sock, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move { socks4_parser(sock).await.ok() });
    }
}
