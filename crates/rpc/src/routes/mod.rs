use actix_web::{
    Error,
    web::{Bytes, Data},
};
use debug::debug_router;
use eth::eth_router;
use serde_json::Value;
use silius_storage::db::SiliusDB;
use web3::web3_router;

use crate::types::{JSONRPC_VERSION, error::ErrorData, request::Request, response::Response};

pub mod debug;
pub mod eth;
pub mod web3;

pub async fn rpc_router(body: Bytes, db: Data<SiliusDB>) -> Result<Bytes, Error> {
    let request: Request = match serde_json::from_slice(body.as_ref()) {
        Ok(ok) => ok,
        Err(_) => {
            let r = Response {
                jsonrpc: String::from(JSONRPC_VERSION),
                result: Value::Null,
                error: Some(ErrorData::std(-32700)),
                id: Value::Null,
            };
            return Ok(r.dump().into());
        }
    };

    let mut response = Response {
        id: request.id.clone(),
        ..Response::default()
    };

    match rpc_select(&request.method.as_str(), request.params, &db).await {
        Ok(ok) => response.result = ok,
        Err(e) => response.error = Some(e),
    }

    Ok(response.dump().into())
}

pub async fn rpc_select(
    method: &str,
    params: Vec<Value>,
    db: &SiliusDB,
) -> Result<Value, ErrorData> {
    match method {
        method if method.starts_with("web3") => web3_router(method, params, db).await,
        method if method.starts_with("eth") => eth_router(method, params, db).await,
        method if method.starts_with("debug") => debug_router(method, params, db).await,
        _ => Err(ErrorData::std(-32601)),
    }
}
