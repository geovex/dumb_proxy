pub enum HttpError {
    HeaderToBig,
    HeaderIncomplete,
    HeaderNotUtf8,
    HeaderInvalid,
    TargetUnreachable,
    Tranciever,
    Internal
}