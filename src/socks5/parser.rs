use super::{AuthRequest, ConnectRequest, RequestAddr};
use nom::{
    bytes::streaming::{tag, take},
    error::{make_error, ErrorKind},
    multi::count,
    number::streaming::{be_u16, be_u8},
    Err, IResult,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn parse_auth(input: &[u8]) -> IResult<&[u8], AuthRequest> {
    let (rest, _) = tag([5u8])(input)?;
    let (rest, len) = be_u8(rest)?;
    let (rest, auths) = take(len)(rest)?;
    let request = AuthRequest {
        auths: Vec::from(auths),
    };
    Ok((rest, request))
}

pub(super) fn parse_request(input: &[u8]) -> IResult<&[u8], ConnectRequest> {
    let (rest, _) = tag([5u8])(input)?;
    let (rest, cmd) = be_u8(rest)?;
    let (rest_save, _rsv) = tag([0u8])(rest)?;
    let (rest, addr_type) = be_u8(rest_save)?;
    let (rest, addr) = match addr_type {
        1 => { //v4
            let (rest, ip) = take(4usize)(rest)?;
            (
                rest,
                RequestAddr::Ip(IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]))),
            )
        }
        3 => { //Domain
            let (rest, len) = be_u8(rest)?;
            let (rest, domain) = take(len)(rest)?;
            (
                rest,
                RequestAddr::Domain(String::from_utf8_lossy(domain).into_owned()),
            )
        }
        4 => { //v6
            let (rest, ip) = count(be_u16, 8)(rest)?;
            (
                rest,
                RequestAddr::Ip(IpAddr::V6(Ipv6Addr::new(
                    ip[0], ip[1], ip[2], ip[3], ip[4], ip[5], ip[6], ip[7],
                ))),
            )
        }
        _ => return Err(Err::Error(make_error(rest_save, ErrorKind::Verify))),
    };
    let (rest, dst_port) = be_u16(rest)?;
    let request = ConnectRequest {
        _cmd: cmd,
        addr,
        port: dst_port,
    };
    Ok((rest, request))
}
