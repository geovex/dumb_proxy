use super::{request::Request, headers::Headers, response::Response};
use nom::{
    bytes::complete::{tag, take_until, take_while},
    character::complete::{space1, one_of, alpha1, digit1},
    multi::{fold_many0, many0},
    sequence::tuple,
    IResult,
};
fn token(input: &str) -> IResult<&str, &str> {
    fn token_char(c: char) -> bool {
        const SEPARATOR: &str = "()<>@,;:\\\"/[]?={} \t";
        c > '\x1f' && c != '\x7f' && !(SEPARATOR.contains(c))
    }
    take_while(token_char)(input)
}

fn header_line(input: &str) -> IResult<&str, (&str, String)> {
    fn value(input: &str) -> IResult<&str, String> {
        let (input, result) = take_until("\r\n")(input)?;
        let (input, result_legacy) = fold_many0(
            tuple((tag("\r\n"), one_of("\t "), take_until("\r\n"))),
            String::new(),
            |mut acc, s| {
                acc.push_str(s.0);
                acc.push(s.1);
                acc.push_str(s.2);
                acc
            },
        )(input)?;
        let (input, _) = tag("\r\n")(input)?;
        Ok((input, format!("{}{}", result, result_legacy)))
    }
    let (input, (header, _sep, value)) = tuple((token, tag(": "), value))(input)?;
    Ok((input, (header, value)))
}

pub fn headers(input: &str) -> IResult<&str, Headers> {
    let (input, headers_raw) = many0(header_line)(input)?;
    let mut headers = Headers::new();
    for (k, v) in headers_raw {
        headers.insert_header(k.to_string(), v);
    }
    Ok((input, headers))
}

fn request_first_line(input: &str) -> IResult<&str, (&str, &str, &str)> {
    let (input, (method, _space0, url, _space1, _http, http_version)) = tuple((
        alpha1,
        space1,
        take_until(" "),
        space1,
        tag("HTTP/"),
        take_until("\r\n"),
    ))(input)?;
    let (input, _) = tag("\r\n")(input)?;
    Ok((input, (method, url, http_version)))
}

pub fn request(input: &str) -> IResult<&str, Request> {
    let (input, rfl) = request_first_line(input)?;
    let (input, headers) = headers(input)?;
    let (input, _) = tag("\r\n")(input)?;
    Ok((input, Request::new(rfl.0.to_string(), rfl.1.to_string(), rfl.2.to_string(), headers)))
}

fn response_first_line(input: &str) -> IResult<&str, (&str, &str, &str)> {
    let (input, (_, http_version, _, status, _, phrase, _)) = tuple((
        tag("HTTP/"),
        take_until(" "),
        space1,
        digit1,
        space1, 
        take_until("\r\n"), 
        tag("\r\n")
    ))(input)?;
    Ok((input, (http_version, status, phrase)))
}

pub fn response(input: &str) -> IResult<&str, Response> {
    let (input, (http_version, status, phrase)) = response_first_line(input)?;
    let status:u16 = status.parse().unwrap();
    let (input, headers) = headers(input)?;
    let (input, _) = tag("\r\n")(input)?;
    Ok((input, Response::new(http_version.to_string(), status, phrase.to_string(), headers)))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn token_basic() {
        let (rest, tok) = super::token("abc: rest").unwrap();
        assert_eq!(tok, "abc");
        assert_eq!(rest, ": rest");
    }
    #[test]
    fn header_line_basic() {
        let line = "header: value\r\n";
        let (key, value) = header_line(line).unwrap().1;
        assert_eq!(key, "header");
        assert_eq!(value, "value");
    }
    #[test]
    fn header_line_legacy() {
        let line = "header: value0\r\n\tvalue1\r\n value2\r\n";
        let (key, value) = header_line(line).unwrap().1;
        assert_eq!(key, "header");
        assert_eq!(value, "value0\r\n\tvalue1\r\n value2");
    }
    #[test]
    fn headers_basic() {
        let line = "header0: value0\r\n\tvalue1\r\n value2\r\nheader1: value0\r\nheader2: value0\r\nheader1: value1\r\n";
        let h = headers(line).unwrap().1;
        assert_eq!(
            h.combined_value("header0").unwrap(),
            "value0\r\n\tvalue1\r\n value2".to_string()
        );
        assert_eq!(h.combined_value("header1").unwrap(), "value0, value1");
        assert_eq!(h.combined_value("header2").unwrap(), "value0");
    }
    #[test]
    fn request_first_line_basic() {
        let line = "GET http://example.net:80/ HTTP/1.1\r\n";
        let h = request_first_line(line).unwrap().1;
        assert_eq!(h.0, "GET");
        assert_eq!(h.1, "http://example.net:80/");
        assert_eq!(h.2, "1.1");
    }
    #[test]
    fn request_basic() {
        let line = "GET http://example.net:80/ HTTP/1.1\r\nheader0: value0\r\nheader1: value1\r\n\r\n";
        let (rest, r) = request(line).unwrap();
        assert_eq!(rest, "");
        assert_eq!(r.method, "GET");
        assert_eq!(r.url, "http://example.net:80/");
        assert_eq!(r.http_version, "1.1");
        assert_eq!(r.headers.combined_value("header0").unwrap(), "value0");
    }
    #[test]
    fn response_first_line_basic() {
        let line = "HTTP/1.1 200 OK\r\n";
        let (rest, r) = response_first_line(line).unwrap();
        assert_eq!(rest, "");
        assert_eq!(r, ("1.1", "200", "OK"));
    }
    #[test]
    fn responce_basic() {
        let line = "HTTP/1.1 200 OK\r\nheader0: value0\r\nheader1: value1\r\n\r\n";
        let (rest, r) = response(line).unwrap();
        assert_eq!(rest, "");
        assert_eq!(r.http_version, "1.1");
        assert_eq!(r.status, 200);
        assert_eq!(r.status_phrase, "OK");
        assert_eq!(r.headers.combined_value("header0").unwrap(), "value0")
    }
}
