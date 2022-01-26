#[derive(Debug)]
pub enum HttpError {
    HeaderToBig,
    HeaderIncomplete(String),
    HeaderNotUtf8,
    HeaderInvalid,
    TargetUnreachable(String),
    LimitedTranciever,
    ChunkTranciever,
    Internal
}