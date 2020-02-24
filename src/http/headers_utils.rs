use super::header_value_parser::value_list;

impl super::headers::Headers {
    pub fn is_chuncked(&self) -> bool {
        let te = self
            .combined_value("Transfer-Encoding")
            .unwrap_or(String::new());
        if let Ok(("", list)) = value_list(te.as_str()) {
            list.contains(&"chunked")
        } else {
            false
        }
    }
    pub fn content_length(&self) -> Option<u128> {
        let cl = self.combined_value("Content-Length").unwrap_or(String::new());
        cl.parse().ok()
    }

    pub fn is_keep_alive(&self) -> bool {
        let c = self.combined_value("Connection").unwrap_or(String::new());
        c.to_lowercase() == "keep-alive"
    }
}
