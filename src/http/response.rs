use super::headers::Headers;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref FIRST_RESPONSE_LINE: Regex =
        Regex::new(r"HTTP/(?P<ver>[0-9\.]+) (?P<status>[0-9]+) (?P<phrase>.+)").unwrap();
}

#[derive(Debug, Clone)]
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
