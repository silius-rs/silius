use crate::gen::{EntryPointAPIErrors, FailedOp};
use ethers::{
    abi::AbiDecode,
    providers::{JsonRpcError, Middleware, MiddlewareError, ProviderError},
    types::Bytes,
};
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;

/// Entry point errors
#[derive(Debug, Error, Clone)]
pub enum EntryPointError {
    /// Failed user operation error
    #[error("{0}")]
    FailedOp(FailedOp),

    /// execution reverted
    #[error("execution reverted: {0}")]
    ExecutionReverted(String),

    /// There is no revert when there should be
    #[error("{function} should revert")]
    NoRevert {
        /// function
        function: String,
    },

    /// Provider error
    #[error("provider error: {inner}")]
    Provider {
        /// The inner error message
        inner: String,
    },

    /// ABI error
    #[error("abi error: {inner}")]
    ABI {
        /// The inner error message
        inner: String,
    },

    /// Data decoding error
    #[error("decode error: {inner}")]
    Decode {
        /// The inner error message
        inner: String,
    },

    /// Any other error
    #[error("other error: {inner}")]
    Other {
        /// The inner error message
        inner: String,
    },
}

impl EntryPointError {
    pub fn from_provider_error(err: &ProviderError) -> Result<EntryPointAPIErrors, Self> {
        match err {
            ProviderError::JsonRpcClientError(err) => err
                .as_error_response()
                .map(Self::from_json_rpc_error)
                .unwrap_or(Err(EntryPointError::Provider {
                    inner: format!("unknwon json-rpc client error: {err:?}"),
                })),
            ProviderError::HTTPError(err) => {
                Err(EntryPointError::Provider { inner: format!("HTTP error: {err:?}") })
            }
            _ => {
                Err(EntryPointError::Provider { inner: format!("unknown provider error: {err:?}") })
            }
        }
    }

    pub fn from_json_rpc_error(err: &JsonRpcError) -> Result<EntryPointAPIErrors, Self> {
        if let Some(ref value) = err.data {
            match value {
                serde_json::Value::String(data) => {
                    let re = Regex::new(r"0x[0-9a-fA-F]+").expect("Regex rules valid");

                    let hex = if let Some(hex) = re.find(data) {
                        hex
                    } else {
                        return Err(EntryPointError::Decode {
                            inner: format!("hex string not found in {data:?}"),
                        });
                    };

                    let bytes = match Bytes::from_str(hex.into()) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return Err(EntryPointError::Decode {
                                inner: format!(
                                    "string {data:?} could not be converted to bytes: {e:?}",
                                ),
                            })
                        }
                    };

                    let revert_err = decode_revert_error(bytes);
                    match revert_err {
                        Ok(res) => return Ok(res),
                        Err(err) => {
                            return Err(EntryPointError::Provider {
                                inner: format!("failed to decode revert error: {err:?}"),
                            })
                        }
                    };
                }
                other => {
                    return Err(Self::Decode {
                        inner: format!("json-rpc return data is not a string: {other:?}"),
                    })
                }
            }
        }

        Err(Self::Provider { inner: format!("json-rpc error doesn't contain data field: {err:?}") })
    }

    pub fn from_middleware_error<M: Middleware>(
        err: M::Error,
    ) -> Result<EntryPointAPIErrors, Self> {
        if let Some(err) = err.as_error_response() {
            return Self::from_json_rpc_error(err);
        }

        if let Some(err) = err.as_provider_error() {
            return Self::from_provider_error(err);
        }

        Err(Self::Provider { inner: format!("middleware error: {err:?}") })
    }
}

// ethers-rs could not handle `require (true, "reason")` or `revert("test failed")` well in this
// case revert with `require` error would ends up with error event signature `0x08c379a0`
// we need to handle it manually
pub fn decode_revert_string(data: Bytes) -> Option<String> {
    let (error_sig, reason) = data.split_at(4);
    if error_sig == [0x08, 0xc3, 0x79, 0xa0] {
        <String as AbiDecode>::decode(reason).ok()
    } else {
        None
    }
}

pub fn decode_revert_error(data: Bytes) -> Result<EntryPointAPIErrors, EntryPointError> {
    let decoded = EntryPointAPIErrors::decode(data.as_ref());
    match decoded {
        Ok(res) => Ok(res),
        Err(e) => {
            if let Some(error_str) = decode_revert_string(data) {
                return Ok(EntryPointAPIErrors::RevertString(error_str));
            };

            Err(EntryPointError::Decode {
                inner: format!(
                    "data field can't be deserialized to EntryPointAPIErrors error: {e:?}",
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_error_msg() -> eyre::Result<()> {
        let err_msg = Bytes::from_str("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001841413934206761732076616c756573206f766572666c6f770000000000000000")?;
        let res = EntryPointAPIErrors::decode(err_msg)?;
        match res {
            EntryPointAPIErrors::RevertString(s) => {
                assert_eq!(s, "AA94 gas values overflow")
            }
            _ => panic!("Invalid error message"),
        }

        let err_msg = Bytes::from_str("0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001841413934206761732076616c756573206f766572666c6f770000000000000000")?;
        let res = EntryPointAPIErrors::decode(err_msg);
        assert!(
            matches!(res, Err(_)),
            "ethers-rs derivatives could not handle revert error correctly"
        );
        Ok(())
    }

    #[test]
    fn deserialize_failed_op() -> eyre::Result<()> {
        let err_msg = Bytes::from_str("0x220266b600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000001e41413430206f76657220766572696669636174696f6e4761734c696d69740000")?;
        let res = EntryPointAPIErrors::decode(err_msg)?;
        match res {
            EntryPointAPIErrors::FailedOp(f) => {
                assert_eq!(f.reason, "AA40 over verificationGasLimit")
            }
            _ => panic!("Invalid error message"),
        }
        Ok(())
    }
}
