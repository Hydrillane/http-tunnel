use derive_builder::Builder;
use serde::Serialize;


#[derive(Builder,Copy,Clone,Default,Serialize)]
pub struct TunnelCtx {
    id: u128,
}

#[derive(Clone,Eq,PartialEq, Debug, Serialize)]
pub enum EstablishTunnelResult {
    Ok,
    OkWithNugget,
    BadRequest,
    Forbidden,
    OperationNotAllowed,
    RequestTimeout,
    BadGateway,
    GatewayTimeout,
    TooManyRequest,
    ServerError,
}


