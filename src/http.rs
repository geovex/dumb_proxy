use lazy_static::lazy_static;
use regex::Regex;
use std::io;
use tokio::io::Result;
use tokio::net::{TcpListener, TcpStream};
use super::util;

lazy_static! {
    static ref FIRST_HEADER_LINE: Regex =
        Regex::new(r"(?P<method>GET|POST|CONNECT) (?P<url>[^ ]+)( HTTP/(?P<ver>[0-9\.]+))?")
            .unwrap();
    static ref HEADER: Regex = Regex::new(r"(?P<key>[^:]+): (?P<value>.*)").unwrap();
}

const INITIAL_HEADER_CAPACITY: usize = 512;
const MAX_HEADER_HEADER_CAPACITY: usize = 64 * 1024;

#[derive(Debug, Clone)]
struct Request {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    fn from_string(request: String) -> Option<Request> {
        let lines: Vec<&str> = request.split("\r\n").collect();
        if lines.len() < 3 {
            return None;
        };
        let lines = &lines[0..lines.len() - 2]; //remove last empty lines
        let firstline_captures = FIRST_HEADER_LINE.captures(lines[0])?;
        //parse headers
        let mut headers = Vec::new();
        for line in &lines[1..] {
            let captures = HEADER.captures(line)?;
            headers.push((captures["key"].into(), captures["value"].into()));
        }
        Some(Request {
            method: firstline_captures["method"].into(),
            url: firstline_captures["url"].into(),
            http_version: firstline_captures
                .name("ver")
                .map_or("", |m| m.as_str())
                .into(),
            headers: headers,
        })
    }
}

async fn http_parser(mut sock: TcpStream) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    //read header
    sock.set_nodelay(true)?;
    let mut header = Vec::with_capacity(INITIAL_HEADER_CAPACITY);
    while !(header.len() > 4 && header[header.len() - 4..] == b"\r\n\r\n"[..]) {
        header.push(sock.read_u8().await?);
        if header.len() > MAX_HEADER_HEADER_CAPACITY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "header too long",
            ));
        }
    }
    let request = String::from_utf8(header).or(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "request not utf8",
    )))?;
    let request = Request::from_string(request).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid request",
    ))?;
    dbg!(&request);
    //analyze request
    match request.method.as_str() {
        "GET" => unimplemented!(),
        "POST" => unimplemented!(),
        "CONNECT" => {
            let sock_addr = util::resolve_sockaddr(&request.url).await?;
            dbg!(&sock_addr);
            let dst_sock = TcpStream::connect(&sock_addr).await?;
            let reply = format!("HTTP/{} 200 OK\r\n\r\n", request.http_version);
            sock.write_all(reply.as_bytes()).await?;
            util::tcp_tranciever(sock, dst_sock).await?;
            //FIXME handle errors
            //FIXME handle keepalive
        }
        _ => unimplemented!(),
    }
    //println!("{}", request);
    Ok(())
}

pub async fn http(src_port: u16) {
    let mut listener = TcpListener::bind(("0.0.0.0", src_port)).await.unwrap();
    loop {
        let (sock, _addr) = listener.accept().await.unwrap();
        tokio::spawn(async move { http_parser(sock).await });
    }
}
