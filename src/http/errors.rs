#[derive(Debug)]
pub enum HttpError {
    HeaderToBig,
    HeaderIncomplete,
    HeaderNotUtf8,
    HeaderParseError,
    ResponceHeaderParseError,
    UrlProtocolInvalid,
    TargetUnreachable(String),
    LimitedTranciever,
    LimitedTrancieverRead,
    LimitedTrancieverWrite,
    LineRead,
    LineTooLong,
    LineNotUtf8,
    ChunkTranciever,
    Internal
}