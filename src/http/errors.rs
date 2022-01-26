#[derive(Debug)]
pub enum HttpError {
    HeaderToBig,
    HeaderIncomplete(String),
    HeaderNotUtf8,
    HeaderInvalid,
    Resolve(String),
    TargetUnreachable(String),
    LimitedTranciever,
    ChunkTranciever,
    Internal
}