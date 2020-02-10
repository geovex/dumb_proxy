use std::fmt;
#[derive(Clone)]
pub struct Headers {
    headers: Vec<(String, String)>,
}

impl Headers {
    pub fn new() -> Headers {
        Headers { headers: Vec::new() }
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for header in &self.headers {
            result += format!("{}: {}\r\n", header.0, header.1).as_str();
        }
        result
    }

    pub fn combined_value<S: AsRef<str>>(&self, key: S) -> Option<String> {
        let key = key.as_ref().to_string();
        let mut result = String::new();
        for (_k, v) in self.headers.iter().filter(|(k, _v)| *k == key) {
            result += v;
            result += ", ";
        }
        if result.len() > 0 {
            result = result[..result.len() - 2].to_string();
            Some(result)
        } else {
            None
        }
    }

    pub fn insert_header<S1, S2>(&mut self, key: S1, value: S2)
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        self.headers
            .push((key.as_ref().to_string(), value.as_ref().to_string()))
    }
}

impl fmt::Debug for Headers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for h in &self.headers[..self.headers.len()-1] {
            writeln!(f, "{}: {}", h.0, h.1)?;
        }
        let h = &self.headers[self.headers.len()-1];
        write!(f, "{}: {}", h.0, h.1)
    }
}

use regex::Regex;
use lazy_static::lazy_static;
lazy_static!{
    static ref CHUNKED: Regex = Regex::new(r"(^| |,)chunked($| |,)").unwrap();
}

impl Headers {
    pub fn is_chuncked(&self) -> bool {
        let te = self.combined_value("Transfer-Encoding").unwrap_or(String::new());
        CHUNKED.captures(te.as_str()).is_some()
    }
    pub fn content_length(&self) -> Option<u128> {
        let cl = self.combined_value("Content-Length").unwrap_or(String::new());
        cl.parse().ok()
    }
}
