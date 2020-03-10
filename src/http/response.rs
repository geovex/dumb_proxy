use super::{headers::Headers, request::Request};
use std::fmt;

#[derive(Clone)]
pub struct Response {
    pub http_version: String,
    pub status: u16,
    pub status_phrase: String,
    pub headers: Headers,
}

impl Response {
    pub fn new<S1, S2>(
        http_version: S1,
        status: u16,
        status_phrase: S2,
        headers: Headers,
    ) -> Response
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Response {
            http_version: http_version.as_ref().to_string(),
            status,
            status_phrase: status_phrase.as_ref().to_string(),
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

    pub fn has_body(&self, request: &Request) -> bool {
        !(request.method == "HEAD"
            || { self.status >= 100 } && (self.status < 200)
            || self.status == 204
            || self.status == 304)
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{} {} {}",
            self.http_version, self.status, self.status_phrase
        )?;
        write!(f, "{:?}", self.headers)
    }
}
