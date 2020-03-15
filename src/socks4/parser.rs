use nom::{
    bytes::complete::{tag, take},
    number::complete::{be_u16, be_u8},
    sequence::tuple,
    IResult, combinator::all_consuming,
};
use std::net::{Ipv4Addr};

/// parse first 8 bytes in socks header
/// returns cmd, dst_port, dst_ip,
pub fn pre_parser(input: &[u8]) -> IResult<&[u8], (u8, u16, Ipv4Addr)> {
    let (input, (_ver, cmd, dst_port, dst_ip)) =
        all_consuming(tuple((tag(b"\x04"), be_u8, be_u16, take(4usize))))(input)?;
    let dst_ip = Ipv4Addr::new(dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3]);
    Ok((input, (cmd, dst_port, dst_ip)))
}
