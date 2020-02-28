use super::util;
use std::io;
use tokio;
use tokio::io::Result as IoResult;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub mod errors;
use errors::HttpError;
mod connection_pool;
mod header_value_parser;
mod headers;
mod headers_utils;
pub(self) mod parser;
mod request;
mod response;

const INITIAL_HEADER_CAPACITY: usize = 512;
const MAX_HEADER_HEADER_CAPACITY: usize = 64 * 1024;

type HttpResult<T> = Result<T, HttpError>;

async fn limited_transceiver<R, W>(src: &mut W, dst: &mut R, mut limit: u128) -> HttpResult<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut dst_buf = [0u8; 2000];
    while limit > 0 {
        let limited_value = (if limit > 2000 { 2000 } else { limit }) as usize;
        let size = dst
            .read(&mut dst_buf[..limited_value])
            .await
            .or(Err(HttpError::Tranciever))?;
        if size == 0 {
            return Ok(());
        };
        src.write_all(&mut dst_buf[..size])
            .await
            .or(Err(HttpError::Tranciever))?;
        limit -= size as u128;
    }
    Ok(())
}

async fn chunked_transceiver<R, W>(src: &mut W, dst: &mut R) -> HttpResult<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        //read_chunk_size
        let length_str = read_line(dst).await.or(Err(HttpError::Tranciever))?;
        if length_str.len() == 0 {
            return Ok(());
        }
        let length =
            u128::from_str_radix(length_str.as_str(), 16).or(Err(HttpError::Tranciever))?;
        if length == 0 {
            src.write_all(b"0\r\n\r\n")
                .await
                .or(Err(HttpError::Tranciever))?;
            return Ok(());
        } else {
            src.write_all(length_str.as_bytes()).await.unwrap();
            src.write_all(b"\r\n")
                .await
                .or(Err(HttpError::Tranciever))?;
            limited_transceiver(src, dst, length + 2)
                .await
                .or(Err(HttpError::Tranciever))?;
        }
    }
}

async fn read_header<R>(sock: &mut R) -> HttpResult<String>
where
    R: AsyncRead + Unpin,
{
    let mut header = Vec::with_capacity(INITIAL_HEADER_CAPACITY);
    while !(header.len() > 4 && header[header.len() - 4..] == b"\r\n\r\n"[..]) {
        header.push(sock.read_u8().await.or(Err(HttpError::HeaderIncomplete))?);
        if header.len() > MAX_HEADER_HEADER_CAPACITY {
            return Err(HttpError::HeaderToBig);
        }
    }
    Ok(String::from_utf8(header).or(Err(HttpError::HeaderNotUtf8))?)
}
async fn read_line<R>(sock: &mut R) -> IoResult<String>
where
    R: AsyncRead + Unpin,
{
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

async fn http_parser(mut sock: TcpStream) -> HttpResult<()> {
    //read header
    sock.set_nodelay(true).or(Err(HttpError::Internal))?;
    let mut connection_pool = connection_pool::ConnectionPool::new();
    'main: loop {
        let header = read_header(&mut sock).await?;
        let (_input, request) =
            parser::request(header.as_str()).or(Err(HttpError::HeaderInvalid))?;
        dbg!(&request);
        //analyze request
        match request.method.as_str() {
            "GET" => {
                let (_rest, url) = parser::url(request.url.as_str())
                    .or(Err(HttpError::HeaderInvalid))?;
                let to_resolve = format!("{}:{}", url.host, url.port);
                let mut dst = connection_pool
                    .connect_or_reuse(to_resolve)
                    .await
                    .or(Err(HttpError::TargetUnreachable))?;
                //modify request
                let mut new_request = request.clone();
                new_request.url = url.path;
                //send request
                dst.write_all(new_request.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                //check response
                let response = read_header(&mut *dst).await?;
                let (_input, response) =
                    parser::response(response.as_str()).or(Err(HttpError::HeaderInvalid))?;
                dbg!(&response);
                sock.write_all(response.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                //check response format (contet-length or chunked)
                if let Some(length) = response.headers.content_length() {
                    limited_transceiver(&mut sock, &mut *dst, length).await?;
                } else if response.headers.is_chuncked() {
                    chunked_transceiver(&mut sock, &mut *dst).await?;
                }
                if !(request.headers.is_keep_alive() && response.headers.is_keep_alive()) {
                    break 'main;
                }
            }
            "POST" => {
                let (_rest, url) = parser::url(request.url.as_str())
                    .or(Err(HttpError::HeaderInvalid))?;
                let to_resolve = format!("{}:{}", url.host, url.port);
                let mut dst = connection_pool
                    .connect_or_reuse(to_resolve)
                    .await
                    .or(Err(HttpError::TargetUnreachable))?;
                //modify request
                let mut new_request = request.clone();
                new_request.url = url.path;
                //dst.write_all()
                dst.write_all(new_request.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                // check request format (content-length or chunked)
                if let Some(length) = request.headers.content_length() {
                    limited_transceiver(&mut *dst, &mut sock, length).await?;
                } else if request.headers.is_chuncked() {
                    chunked_transceiver(&mut *dst, &mut sock).await?;
                }
                //process response
                let response_header = read_header(&mut *dst).await?;
                let (_input, response) =
                    parser::response(response_header.as_str()).or(Err(HttpError::HeaderInvalid))?;
                dbg!(&response);
                sock.write_all(response.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                //check response format (contet-length or chunked)
                if let Some(length) = response.headers.content_length() {
                    limited_transceiver(&mut sock, &mut *dst, length).await?;
                } else if response.headers.is_chuncked() {
                    chunked_transceiver(&mut sock, &mut *dst).await?;
                }
                if !(request.headers.is_keep_alive() && response.headers.is_keep_alive()) {
                    break 'main;
                }
            }
            "CONNECT" => {
                let sock_addr = util::resolve_sockaddr(&request.url)
                    .await
                    .or(Err(HttpError::TargetUnreachable))?;
                dbg!(&sock_addr);
                let mut dst_sock = TcpStream::connect(&sock_addr)
                    .await
                    .or(Err(HttpError::TargetUnreachable))?;
                let reply = format!("HTTP/{} 200 OK\r\n\r\n", request.http_version);
                sock.write_all(reply.as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                util::tcp_transceiver(&mut sock, &mut dst_sock)
                    .await
                    .or(Err(HttpError::Tranciever))?;
                //FIXME handle errors
                //FIXME handle keepalive
                break;
            }
            _ => return Err(HttpError::HeaderInvalid)
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
