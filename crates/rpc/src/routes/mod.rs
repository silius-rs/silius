use actix_web::{
    Error,
    web::{Bytes, Data},
};
use alloy_provider::Provider;
use debug::debug_router;
use eth::eth_router;
use serde_json::Value;
use silius_manager::SiliusManager;
use web3::web3_router;

use crate::types::{JSONRPC_VERSION, error::ErrorData, request::Request, response::Response};

pub mod debug;
pub mod eth;
pub mod web3;

pub async fn rpc_router<P: Provider + Clone + 'static>(
    body: Bytes,
    api_modules: Data<Vec<String>>,
    manager: Data<SiliusManager<P>>,
) -> Result<Bytes, Error> {
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

    match rpc_select(
        request.method.as_str(),
        request.params,
        api_modules,
        &manager,
    )
    .await
    {
        Ok(ok) => response.result = ok,
        Err(e) => response.error = Some(e),
    }

    Ok(response.dump().into())
}

pub async fn rpc_select<P: Provider + Clone + 'static>(
    method: &str,
    params: Vec<Value>,
    api_modules: Data<Vec<String>>,
    manager: &SiliusManager<P>,
) -> Result<Value, ErrorData> {
    let module = method.split('_').next().unwrap_or_default();
    if !api_modules.contains(&module.to_string()) {
        return Err(ErrorData::std(-32601));
    }

    match method {
        method if method.starts_with("web3") => web3_router(method, params, manager).await,
        method if method.starts_with("eth") => eth_router(method, params, manager).await,
        method if method.starts_with("debug") => debug_router(method, params, manager).await,
        _ => Err(ErrorData::std(-32601)),
    }
}
