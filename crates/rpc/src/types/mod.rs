//! https://github.com/actix/examples/blob/master/json/jsonrpc/src/convention.rs

pub mod codes;
pub mod error;
pub mod request;
pub mod response;

pub static JSONRPC_VERSION: &str = "2.0";
