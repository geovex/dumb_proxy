use super::util;
use crate::config_loader::HttpConfig;
use crate::logger;
use std::time::Duration;
use tokio;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_io_timeout::TimeoutStream;

pub mod errors;
use errors::HttpError;
use headers::Headers;
use response::Response;
mod connection_pool;
mod header_value_parser;
mod headers;
mod headers_utils;
pub(self) mod parser;
mod request;
mod response;

const INITIAL_HEADER_CAPACITY: usize = 512;
const MAX_HEADER_HEADER_CAPACITY: usize = 64 * 1024;
const MAX_LINE_SIZE: usize = 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 120;
const TIMEOUT_TOLERANCE_SECS: u64 = 10;
type HttpResult<T> = Result<T, HttpError>;
const ERROR_400: &str = std::include_str!("error_pages/400.html");
const ERROR_502: &str = std::include_str!("error_pages/502.html");

pub struct Http {
    name: String,
    config: HttpConfig,
}

impl Http {
    pub fn new(name: &String, config: &HttpConfig) -> Http {
        Http {
            name: name.clone(),
            config: config.clone(),
        }
    }

    pub async fn serve(&self) {
        let listener = util::bind_listener(self.config.port).await;
        loop {
            let (sock, _addr) = listener.accept().await.unwrap();
            let name_clone = self.name.clone();
            tokio::spawn(async move { Self::http_processor(name_clone, sock).await });
        }
    }

    async fn http_processor(name: String, sock: TcpStream) {
        match Self::http_parser(name, sock).await {
            Ok(_) => (),
            Err(e) => {
                logger::log(format!("client error: {:?}", e));
            }
        }
    }
    async fn limited_transceiver<R, W>(src: &mut W, dst: &mut R, mut limit: usize) -> HttpResult<()>
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
                .or(Err(HttpError::LimitedTrancieverRead))?;
            if size == 0 {
                return Ok(());
            };
            src.write_all(&mut dst_buf[..size])
                .await
                .or(Err(HttpError::LimitedTrancieverWrite))?;
            limit -= size;
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
            let length_str = Self::read_line(dst).await?;
            let (_rest, (length, _ext)) =
                parser::chunk_line(length_str.as_str()).or(Err(HttpError::ChunkTranciever))?;
            if length == 0 {
                Self::read_line(dst).await?;
                src.write_all(b"0\r\n\r\n")
                    .await
                    .or(Err(HttpError::ChunkTranciever))?;
                return Ok(());
            } else {
                src.write_all(length_str.as_bytes()).await.unwrap();
                src.write_all(b"\r\n")
                    .await
                    .or(Err(HttpError::ChunkTranciever))?;
                Self::limited_transceiver(src, dst, length + 2)
                    .await
                    .or(Err(HttpError::ChunkTranciever))?;
            }
        }
    }

    async fn read_header<R>(sock: &mut R) -> HttpResult<String>
    where
        R: AsyncRead + Unpin,
    {
        let mut header = Vec::with_capacity(INITIAL_HEADER_CAPACITY);
        while !(header.ends_with(b"\r\n\r\n")) {
            let byte = sock.read_u8().await.or(Err(HttpError::HeaderIncomplete))?;
            header.push(byte);
            if header.len() > MAX_HEADER_HEADER_CAPACITY {
                return Err(HttpError::HeaderToBig);
            }
        }
        Ok(String::from_utf8(header).or(Err(HttpError::HeaderNotUtf8))?)
    }
    async fn read_line<R>(sock: &mut R) -> HttpResult<String>
    where
        R: AsyncRead + Unpin,
    {
        let mut result = Vec::new();
        loop {
            result.push(sock.read_u8().await.or(Err(HttpError::LineRead))?);
            if result.ends_with(b"\r\n") {
                break;
            } else if result.len() > MAX_LINE_SIZE {
                return Err(HttpError::LineTooLong);
            }
        }
        result.resize(result.len() - 2, 0);
        Ok(String::from_utf8(result).or(Err(HttpError::LineNotUtf8))?)
    }

    async fn return_error_page<W, S>(src: &mut W, mut response: Response, body: S) -> HttpResult<()>
    where
        W: AsyncWrite + Unpin,
        S: AsRef<str>,
    {
        let bytes = body.as_ref().as_bytes();
        response
            .headers
            .insert_header("Content-Length", bytes.len().to_string());
        response.headers.insert_header("Content-Type", "text/html");
        src.write_all(response.to_string().as_bytes())
            .await
            .or(Err(HttpError::Internal))?;
        src.write_all(bytes).await.or(Err(HttpError::Internal))?;
        Ok(())
    }

    async fn http_parser(name: String, sock: TcpStream) -> HttpResult<()> {
        let src_ip = sock.peer_addr().unwrap();
        //read header
        sock.set_nodelay(true).or(Err(HttpError::Internal))?;
        let mut connection_pool = connection_pool::ConnectionPool::new();
        let mut timed_out_stream = TimeoutStream::new(sock);
        timed_out_stream.set_read_timeout(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)));
        let mut timed_out_stream = Box::pin(timed_out_stream);
        'main: loop {
            let header = Self::read_header(&mut timed_out_stream).await?;
            let request = match parser::request(header.as_str()) {
                Ok((_rest, request)) => request,
                Err(_) => {
                    let response = Response::new("1.1", 400, "invalid header", Headers::new());
                    Self::return_error_page(&mut timed_out_stream, response, ERROR_400).await?;
                    return Err(HttpError::HeaderParseError);
                }
            };
            //analyze request
            if request.method == "CONNECT" {
                request.headers.keep_alive_value();
                let dst_sock = TcpStream::connect(&request.url)
                    .await
                    .or(Err(HttpError::TargetUnreachable(request.url.clone())))?;
                let dst_ip = dst_sock.peer_addr().unwrap();
                let reply = format!("HTTP/{} 200 OK\r\n\r\n", request.http_version);
                timed_out_stream
                    .write_all(reply.as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                logger::log(format!("http.{} CONECT {:?} -> {:?}", name, src_ip, dst_ip));
                let mut dst_timed_out = TimeoutStream::new(dst_sock);
                dst_timed_out.set_read_timeout(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)));
                let mut dst_timed_out = Box::pin(dst_timed_out);
                util::transceiver(&mut timed_out_stream, &mut dst_timed_out)
                    .await
                    .or(Err(HttpError::LimitedTranciever))?;
                break;
            } else { // other request methods
                // parse request
                let (_rest, url) =
                    parser::url(request.url.as_str()).or(Err(HttpError::HeaderParseError))?;
                if url.protocol != "http" {
                    return Err(HttpError::UrlProtocolInvalid);
                }
                // connect to target
                let to_resolve = format!("{}:{}", url.host, url.port);
                let mut dst = match connection_pool.connect_or_reuse(&to_resolve).await {
                    Ok(sock) => sock,
                    Err(_) => {
                        let response = Response::new(
                            request.http_version,
                            502,
                            "connection failed",
                            Headers::new(),
                        );
                        Self::return_error_page(&mut timed_out_stream, response, ERROR_502).await?;
                        return Err(HttpError::TargetUnreachable(to_resolve));
                    }
                };
                //modify request
                let mut new_request = request.clone();
                new_request.url = url.path;
                dst.write_all(new_request.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                if request.has_body() {
                    // check request format (content-length or chunked)
                    if let Some(length) = request.headers.content_length() {
                        Self::limited_transceiver(&mut *dst, &mut timed_out_stream, length).await?;
                    } else if request.headers.is_chuncked() {
                        Self::chunked_transceiver(&mut *dst, &mut timed_out_stream).await?;
                    }
                }
                //process response
                let response_header = Self::read_header(&mut *dst).await?;
                let (_input, response) = parser::response(response_header.as_str())
                    .or(Err(HttpError::ResponceHeaderParseError))?;
                timed_out_stream
                    .write_all(response.to_string().as_bytes())
                    .await
                    .or(Err(HttpError::Internal))?;
                //update timeout values
                if let Some(timeout) = response.headers.keep_alive_value() {
                    timed_out_stream
                        .as_mut()
                        .set_read_timeout_pinned(Some(Duration::from_secs(
                            timeout.timeout + TIMEOUT_TOLERANCE_SECS,
                        )))
                } else {
                    timed_out_stream
                        .as_mut()
                        .set_read_timeout_pinned(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)))
                }
                logger::log(format!(
                    "http.{} {} {:?} -> {:?} {}",
                    name, request.method, src_ip, request.url, response.status
                ));
                if response.has_body(&request) {
                    //check response format (contet-length or chunked)
                    if let Some(length) = response.headers.content_length() {
                        Self::limited_transceiver(&mut timed_out_stream, &mut *dst, length + 2)
                            .await?;
                    } else if response.headers.is_chuncked() {
                        Self::chunked_transceiver(&mut timed_out_stream, &mut *dst).await?;
                    }
                }
                if !(request.headers.is_keep_alive() && response.headers.is_keep_alive()) {
                    break 'main;
                }
            }
        }
        Ok(())
    }
}
