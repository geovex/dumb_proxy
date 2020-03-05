use super::headers::Headers;
use std::fmt;

#[derive(Clone)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Headers,
}

impl Request {
    pub fn new(method: String, url: String, http_version: String, headers: Headers) -> Request {
        Request {
            method,
            url,
            http_version,
            headers,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{} {} HTTP/{}\r\n{}\r\n",
            self.method,
            self.url,
            self.http_version,
            self.headers.to_string()
        )
    }

    pub fn has_body(&self) -> bool {
        self.method == "POST"
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {} HTTP/{}", self.method, self.url, self.http_version)?;
        write!(f, "{:?}", self.headers)
    }
}

pub struct Url{
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub path: String
}