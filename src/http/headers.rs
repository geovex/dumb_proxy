use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref HEADER: Regex = Regex::new(r"(?P<key>[^:]+): (?P<value>.*)").unwrap();
}

#[derive(Debug, Clone)]
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

//tests
