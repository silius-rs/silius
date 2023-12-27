use ethers::types::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Request<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateResult {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: T,
}

pub mod simple_account {
    use alloy_primitives::{Address, U256};
    use alloy_sol_types::{sol, SolCall};
    use ethers::types::{Address as EAddress, Bytes as EBytes, U256 as EU256};

    sol! {function execute(address dest, uint256 value, bytes calldata func);}

    pub struct SimpleAccountExecute(executeCall);

    impl SimpleAccountExecute {
        pub fn new(address: EAddress, value: EU256, func: EBytes) -> Self {
            Self(executeCall {
                dest: Address::from(address.0),
                value: U256::from_limbs(value.0),
                func: func.to_vec(),
            })
        }

        pub fn encode(&self) -> Vec<u8> {
            self.0.abi_encode()
        }
    }
}
