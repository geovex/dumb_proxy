use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag},
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::recognize,
    multi::separated_list,
    sequence::delimited,
    IResult,
};

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
        recognize(separated_list(multispace1, literal)),
        multispace0,
    )(input)
}

pub fn value_list(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list(tag(","), spaced_item)(input)
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
}
