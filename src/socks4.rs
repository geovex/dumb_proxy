use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, Result};
use tokio::io;
use super::util;

async fn socks4_parser(mut sock: TcpStream) -> Result<()> {
    const GOOD_REPLY: [u8; 8] = [
        0x00u8, //VN
        0x5a, //Granted
        0x00, 0x00, //DSTPORT, 
        0x00, 0x00, 0x00, 0x00]; //DSTIP
    const BAD_REPLY: [u8; 8] = [
        0x00u8, //VN
        0x5b, //Failed
        0x00, 0x00, //DSTPORT, 
        0x00, 0x00, 0x00, 0x00]; //DSTIP
    use tokio::io::AsyncReadExt;
    sock.set_nodelay(true).unwrap();
    //read header
    let mut header = [0u8; 8];
    if 8 != sock.read_exact(&mut header).await? {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "header incomplete"))
    };
    //read_id
    while sock.read_u8().await.unwrap() != 0 {
    };
    //parse header
    if header[0..2] != [4, 1] {
        eprintln!("wrong socks command");
        sock.write_all(&BAD_REPLY).await.ok();
        return Err(io::Error::new(io::ErrorKind::InvalidData, "wrong header"))
    }; //bad socks command
    let port = ((header[2] as u16) << 8) + header[3] as u16;
    let addr = [header[4], header[5], header[6], header[7]];
    let dst_addr = SocketAddr::new(addr.into(), port);
    dbg!(dst_addr);
    let dst = TcpStream::connect(&dst_addr).await;
    if dst.is_ok() {
        sock.write_all(&GOOD_REPLY).await?;
        util::tcp_tranciever(sock, dst.unwrap()).await.ok();
    } else {
        sock.write_all(&BAD_REPLY).await.ok();
    };
    Ok(())
}

pub async fn socks4(src_port: u16) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (sock, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {socks4_parser(sock).await.ok()});
    }
}