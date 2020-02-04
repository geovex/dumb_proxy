use super::util;
use lazy_static::lazy_static;
use regex::Regex;
use std::io;
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};
use tokio::net::{TcpListener, TcpStream};

lazy_static! {
    static ref FIRST_REQUEST_LINE: Regex =
        Regex::new(r"(?P<method>GET|POST|CONNECT) (?P<url>[^ ]+)( HTTP/(?P<ver>[0-9\.]+))?")
            .unwrap();
    static ref FIRST_RESPONSE_LINE: Regex =
        Regex::new(r"HTTP/(?P<ver>[0-9\.]+) (?P<status>[0-9]+) (?P<phrase>.+)").unwrap();
    static ref HTTP_URL: Regex =
        Regex::new(r"http://(?P<domain>[^ :/]+)(?P<port>:[0-9]+)?(?P<path>/[^ ]*)").unwrap();
    static ref CHUNKED: Regex = Regex::new(r"(^| |,)chunked($| |,)").unwrap();
}

mod headers;
use headers::Headers;

const INITIAL_HEADER_CAPACITY: usize = 512;
const MAX_HEADER_HEADER_CAPACITY: usize = 64 * 1024;

#[derive(Debug, Clone)]
struct Request {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Headers,
}

impl Request {
    fn from_string(request: String) -> Option<Request> {
        let lines: Vec<&str> = request.split("\r\n").collect();
        if lines.len() < 3 {
            return None;
        };
        let lines = &lines[0..lines.len() - 2]; //remove last empty lines
        let firstline_captures = FIRST_REQUEST_LINE.captures(lines[0])?;
        //parse headers
        let headers = Headers::from_lines(&lines[1..])?;
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

    fn to_string(&self) -> String {
        format!(
            "{} {} HTTP/{}\r\n{}\r\n",
            self.method,
            self.url,
            self.http_version,
            self.headers.to_string()
        )
    }
}

#[derive(Debug, Clone)]
struct Response {
    pub http_version: String,
    pub status: u16,
    pub status_phrase: String,
    pub headers: Headers,
}

impl Response {
    fn from_string(response: String) -> Option<Response> {
        let lines: Vec<&str> = response.split("\r\n").collect();
        if lines.len() < 3 {
            return None;
        };
        let lines = &lines[0..lines.len() - 2]; //remove last empty line
        let firstline_captures = FIRST_RESPONSE_LINE.captures(lines[0])?;
        //parse headers
        let headers = Headers::from_lines(&lines[1..])?;
        Some(Response {
            http_version: firstline_captures["ver"].into(),
            status: firstline_captures["status"].parse().ok()?,
            status_phrase: firstline_captures["phrase"].into(),
            headers: headers,
        })
    }

    fn to_string(&self) -> String {
        format!(
            "HTTP/{} {} {}\r\n{}\r\n",
            self.http_version,
            self.status.to_string(),
            self.status_phrase,
            self.headers.to_string()
        )
    }
}

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

async fn read_header(sock: &mut TcpStream) -> Result<Vec<u8>> {
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
    Ok(header)
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
    let header = read_header(&mut sock).await?;
    let request = String::from_utf8(header).or(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "request not utf8",
    )))?;
    let request = Request::from_string(request).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid request",
    ))?;
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
            let target_sockaddr = util::resolve_sockaddr(&to_resolve).await?;
            let mut dst = TcpStream::connect(target_sockaddr).await?;
            //modify request
            let mut new_request = request.clone();
            new_request.url = target_captures["path"].to_string();
            //send request
            let bin_request = new_request.to_string();
            dst.write_all(bin_request.as_bytes()).await?;
            //check response
            let response = read_header(&mut dst).await?;
            let response = String::from_utf8(response).or(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid utf8 response",
            )))?;
            let response = Response::from_string(response).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid response",
            ))?;
            //check response format (contet-length or chunked)
            if let Some(size_str) = response.headers.combined_value("Content-Length") {
                let length: u128 = size_str.parse().or(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid content length",
                )))?;
                sock.write_all(response.to_string().as_bytes()).await?;
                limited_transiever(&mut sock, &mut dst, length).await?;
            } else if let Some(transfer_encoding) =
                response.headers.combined_value("Transfer-Encoding")
            {
                if CHUNKED.captures(transfer_encoding.as_str()).is_some() {
                    sock.write_all(response.to_string().as_bytes()).await?;
                    chunked_transiever(&mut sock, &mut dst).await?;
                }
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
            let target_sockaddr = util::resolve_sockaddr(&to_resolve).await?;
            let mut dst = TcpStream::connect(target_sockaddr).await?;
            //modify request
            let mut new_request = request.clone();
            new_request.url = target_captures["path"].to_string();
            //dst.write_all()
            // check request format (content-length or chunked)
            if let Some(size_str) = request.headers.combined_value("Content-Length") {
                let length: u128 = size_str.parse().or(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid content-length",
                )))?;
                dst.write_all(new_request.to_string().as_bytes()).await?;
                limited_transiever(&mut dst, &mut sock, length).await?;
            } else if let Some(transfer_encoding) =
                request.headers.combined_value("Transfer-Encoding")
            {
                dst.write_all(new_request.to_string().as_bytes()).await?;
                if CHUNKED.captures(transfer_encoding.as_str()).is_some() {
                    chunked_transiever(&mut dst, &mut sock).await?;
                }
            }
            //process response
            let response_header = read_header(&mut dst).await?;
            let responce = String::from_utf8(response_header).or(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "responce not utf8",
            )))?;
            let response = Response::from_string(responce).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid response",
            ))?;
            //check response format (contet-length or chunked)
            if let Some(size_str) = response.headers.combined_value("Content-Length") {
                let length: u128 = size_str.parse().or(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid content length",
                )))?;
                sock.write_all(response.to_string().as_bytes()).await?;
                limited_transiever(&mut sock, &mut dst, length).await?;
            } else if let Some(transfer_encoding) =
                response.headers.combined_value("Transfer-Encoding")
            {
                sock.write_all(response.to_string().as_bytes()).await?;
                if CHUNKED.captures(transfer_encoding.as_str()).is_some() {
                    chunked_transiever(&mut sock, &mut dst).await?;
                }
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
