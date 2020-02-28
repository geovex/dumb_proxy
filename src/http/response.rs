use super::headers::Headers;
use lazy_static::lazy_static;
use std::fmt;

#[derive(Clone)]
pub struct Response {
    pub http_version: String,
    pub status: u16,
    pub status_phrase: String,
    pub headers: Headers,
}

impl Response {
    pub fn new(
        http_version: String,
        status: u16,
        status_phrase: String,
        headers: Headers,
    ) -> Response {
        Response {
            http_version,
            status,
            status_phrase,
            headers,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "HTTP/{} {} {}\r\n{}\r\n",
            self.http_version,
            self.status.to_string(),
            self.status_phrase,
            self.headers.to_string()
        )
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {} {}", self.http_version, self.status, self.status_phrase)?;
        write!(f, "{:?}", self.headers)
    }
}