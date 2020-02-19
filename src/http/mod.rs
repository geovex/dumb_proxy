use super::util;
use lazy_static::lazy_static;
use regex::Regex;
use std::io;
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};
use tokio::net::{TcpListener, TcpStream};

lazy_static! {
    static ref HTTP_URL: Regex =
        Regex::new(r"http://(?P<domain>[^ :/]+)(?P<port>:[0-9]+)?(?P<path>/[^ ]*)").unwrap();
    static ref CHUNKED: Regex = Regex::new(r"(^| |,)chunked($| |,)").unwrap();
}

mod headers;
mod header_value_parser;
mod request;
mod response;
mod connection_pool;
pub(self) mod parser;

const INITIAL_HEADER_CAPACITY: usize = 512;
const MAX_HEADER_HEADER_CAPACITY: usize = 64 * 1024;


async fn limited_transiever(
    src: &mut TcpStream,
    dst: &mut TcpStream,
    mut limit: u128,
) -> Result<()> {
    let mut dst_buf = [0u8; 2000];
    while limit > 0 {
        let limited_value = (if limit > 2000 { 2000 } else { limit }) as usize;
        let size = dst.read(&mut dst_buf[..limited_value]).await?;
        if size == 0 {
            return Ok(());
        };
        src.write_all(&mut dst_buf[..size]).await?;
        limit -= size as u128;
    }
    Ok(())
}

async fn chunked_transiever(src: &mut TcpStream, dst: &mut TcpStream) -> Result<()> {
    loop {
        //read_chunk_size
        let length_str = read_line(dst).await?;
        if length_str.len() == 0 {
            return Ok(());
        }
        let length = u128::from_str_radix(length_str.as_str(), 16).or(Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "wrong chunked length",
        )))?;
        if length == 0 {
            src.write_all(b"0\r\n\r\n").await?;
            return Ok(());
        } else {
            src.write_all(length_str.as_bytes()).await.unwrap();
            src.write_all(b"\r\n").await?;
            limited_transiever(src, dst, length+2).await?;
        }
    }
}

async fn read_header(sock: &mut TcpStream) -> Result<String> {
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
    Ok(String::from_utf8(header).or(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "header not utf8",
    )))?)
}
async fn read_line(sock: &mut TcpStream) -> Result<String> {
    let mut result = Vec::new();
    loop {
        result.push(sock.read_u8().await?);
        if result.len() > 2 && result[result.len() - 2..] == b"\r\n"[..] {
            break;
        }
    }
    result.resize(result.len() - 2, 0);
    Ok(String::from_utf8(result).or(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "bad utf8 line",
    )))?)
}

async fn http_parser(mut sock: TcpStream) -> Result<()> {
    //read header
    sock.set_nodelay(true)?;
    let mut connection_pool = connection_pool::ConnectionPool::new();
    'main: loop {
        let header = read_header(&mut sock).await?;
        let (_input, request) = parser::request(header.as_str()).or(Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid request",
        )))?;
        dbg!(&request);
        //analyze request
        match request.method.as_str() {
            "GET" => {
                let target_captures = HTTP_URL
                    .captures(request.url.as_str())
                    .ok_or(io::Error::new(io::ErrorKind::InvalidData, "invalid url"))?;
                let to_resolve = format!(
                    "{}{}",
                    &target_captures["domain"],
                    target_captures.name("port").map_or(":80", |m| m.as_str())
                );
                let mut dst = connection_pool.connect_or_reuse(to_resolve).await?;
                //modify request
                let mut new_request = request.clone();
                new_request.url = target_captures["path"].to_string();
                //send request
                let bin_request = new_request.to_string();
                dst.write_all(bin_request.as_bytes()).await?;
                //check response
                let response = read_header(&mut dst).await?;
                let (_input, response) = parser::response(response.as_str()).or(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid response",
                )))?;
                dbg!(&response);
                sock.write_all(response.to_string().as_bytes()).await?;
                //check response format (contet-length or chunked)
                if let Some(length) = response.headers.content_length() {
                    limited_transiever(&mut sock, &mut dst, length).await?;
                } else if response.headers.is_chuncked() {
                    chunked_transiever(&mut sock, &mut dst).await?;
                }
                if !(request.headers.is_keep_alive() && response.headers.is_keep_alive()) {
                    break 'main;
                }
            }
            "POST" => {
                let target_captures = HTTP_URL
                    .captures(request.url.as_str())
                    .ok_or(io::Error::new(io::ErrorKind::InvalidData, "invalid url"))?;
                let to_resolve = format!(
                    "{}{}",
                    &target_captures["domain"],
                    target_captures.name("port").map_or(":80", |m| m.as_str())
                );
                let mut dst = connection_pool.connect_or_reuse(to_resolve).await?;
                //modify request
                let mut new_request = request.clone();
                new_request.url = target_captures["path"].to_string();
                //dst.write_all()
                dst.write_all(new_request.to_string().as_bytes()).await?;
                // check request format (content-length or chunked)
                if let Some(length) = request.headers.content_length() {
                    limited_transiever(&mut dst, &mut sock, length).await?;
                } else if request.headers.is_chuncked() {
                    chunked_transiever(&mut dst, &mut sock).await?;
                }
                //process response
                let response_header = read_header(&mut dst).await?;
                let (_input, response) = parser::response(response_header.as_str()).or(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid response",
                )))?;
                dbg!(&response);
                sock.write_all(response.to_string().as_bytes()).await?;
                //check response format (contet-length or chunked)
                if let Some(length) = response.headers.content_length() {
                    limited_transiever(&mut sock, &mut dst, length).await?;
                } else if response.headers.is_chuncked() {
                    chunked_transiever(&mut sock, &mut dst).await?;
                }
                if !(request.headers.is_keep_alive() && response.headers.is_keep_alive()){
                    break 'main;
                }
            }
            "CONNECT" => {
                let sock_addr = util::resolve_sockaddr(&request.url).await?;
                dbg!(&sock_addr);
                let mut dst_sock = TcpStream::connect(&sock_addr).await?;
                let reply = format!("HTTP/{} 200 OK\r\n\r\n", request.http_version);
                sock.write_all(reply.as_bytes()).await?;
                util::tcp_tranciever(&mut sock, &mut dst_sock).await?;
                //FIXME handle errors
                //FIXME handle keepalive
                break;
            }
            _ => unimplemented!(),
        }
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
