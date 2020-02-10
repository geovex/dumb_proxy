use super::headers::Headers;
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;

lazy_static! {
    static ref FIRST_REQUEST_LINE: Regex =
        Regex::new(r"(?P<method>GET|POST|CONNECT) (?P<url>[^ ]+)( HTTP/(?P<ver>[0-9\.]+))?")
            .unwrap();
    static ref HTTP_URL: Regex =
        Regex::new(r"http://(?P<domain>[^ :/]+)(?P<port>:[0-9]+)?(?P<path>/[^ ]*)").unwrap();
    static ref CHUNKED: Regex = Regex::new(r"(^| |,)chunked($| |,)").unwrap();
}

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
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {} HTTP/{}", self.method, self.url, self.http_version)?;
        write!(f, "{:?}", self.headers)
    }
}