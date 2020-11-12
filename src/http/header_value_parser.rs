use nom::{IResult, branch::alt, bytes::complete::{escaped, is_not, tag}, character::complete::{multispace0, multispace1, none_of, one_of}, combinator::{recognize, rest}, multi::separated_list0, sequence::{delimited, separated_pair}};
use std::collections::BTreeMap;

fn literal(input: &str) -> IResult<&str, &str> {
    alt((
        is_not(" \","),
        recognize(delimited(
            tag("\""),
            escaped(none_of("\\\""), '\\', one_of("\"\\")),
            tag("\""),
        )),
    ))(input)
}

fn spaced_item(input: &str) -> IResult<&str, &str> {
    delimited(
        multispace0,
        recognize(separated_list0(multispace1, literal)),
        multispace0,
    )(input)
}

pub fn value_list(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list0(tag(","), spaced_item)(input)
}

pub fn kv(input: &str) -> IResult<&str, BTreeMap<&str, &str>> {
    fn kv(input: &str) -> IResult<&str, (&str, &str)> {
        separated_pair(is_not("="), tag("="), rest)(input)
    }
    let (input, items) = value_list(input)?;
    let mut result = BTreeMap::new();
    for item in items {
        let (_, (k, v)) = kv(item)?;
        result.insert(k, v);
    }
    Ok((input, result))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn literal_basic() {
        let (_rest, input) = literal("test").unwrap();
        assert_eq!(input, "test");
        let (_rest, input) = literal("\" test string \"").unwrap();
        assert_eq!(input, "\" test string \"");
    }
    #[test]
    fn header_line_basic() {
        let (_rest, p) = value_list("   test0,   test1 ").unwrap();
        assert_eq!(p, ["test0", "test1"]);
        let (_rest, p) = value_list("   test0, test1  test1 ").unwrap();
        assert_eq!(p, ["test0", "test1  test1"]);
        let (_rest, input) = value_list("\" test0 \", \"test1,comma\"").unwrap();
        assert_eq!(input, ["\" test0 \"", "\"test1,comma\""]);
    }
    #[test]
    fn kv_basic() {
        let (_rest, kv) = kv("a=1, b=2, c=3 ").unwrap();
        assert_eq!(kv["a"], "1");
        assert_eq!(kv["b"], "2");
        assert_eq!(kv["c"], "3");
    }
}
