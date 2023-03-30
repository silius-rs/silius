// Code adapted from: https://github.com/ledgerwatch/interfaces/blob/master/src/lib.rs#L1
pub mod types {
    use arrayref::array_ref;
    use ethers::types::{Address, Bytes, U256};

    tonic::include_proto!("types");

    impl From<ethers::types::H128> for H128 {
        fn from(value: ethers::types::H128) -> Self {
            Self {
                hi: u64::from_be_bytes(*array_ref!(value, 0, 8)),
                lo: u64::from_be_bytes(*array_ref!(value, 8, 8)),
            }
        }
    }

    impl From<ethers::types::H160> for H160 {
        fn from(value: ethers::types::H160) -> Self {
            Self {
                hi: Some(ethers::types::H128::from_slice(&value[..16]).into()),
                lo: u32::from_be_bytes(*array_ref!(value, 16, 4)),
            }
        }
    }

    impl From<ethers::types::H256> for H256 {
        fn from(value: ethers::types::H256) -> Self {
            Self {
                hi: Some(ethers::types::H128::from_slice(&value[..16]).into()),
                lo: Some(ethers::types::H128::from_slice(&value[16..]).into()),
            }
        }
    }

    impl From<H128> for ethers::types::H128 {
        fn from(value: H128) -> Self {
            let mut v = [0; Self::len_bytes()];
            v[..8].copy_from_slice(&value.hi.to_be_bytes());
            v[8..].copy_from_slice(&value.lo.to_be_bytes());

            v.into()
        }
    }

    impl From<H160> for ethers::types::H160 {
        fn from(value: H160) -> Self {
            type H = ethers::types::H128;

            let mut v = [0; Self::len_bytes()];
            v[..H::len_bytes()]
                .copy_from_slice(H::from(value.hi.unwrap_or_default()).as_fixed_bytes());
            v[H::len_bytes()..].copy_from_slice(&value.lo.to_be_bytes());

            v.into()
        }
    }

    impl From<H256> for ethers::types::H256 {
        fn from(value: H256) -> Self {
            type H = ethers::types::H128;

            let mut v = [0; Self::len_bytes()];
            v[..H::len_bytes()]
                .copy_from_slice(H::from(value.hi.unwrap_or_default()).as_fixed_bytes());
            v[H::len_bytes()..]
                .copy_from_slice(H::from(value.lo.unwrap_or_default()).as_fixed_bytes());

            v.into()
        }
    }

    impl From<crate::types::user_operation::UserOperation> for UserOperation {
        fn from(user_operation: crate::types::user_operation::UserOperation) -> Self {
            Self {
                sender: Some(user_operation.sender.into()),
                nonce: user_operation.nonce.as_u64(),
                init_code: prost::bytes::Bytes::copy_from_slice(user_operation.init_code.as_ref()),
                call_data: prost::bytes::Bytes::copy_from_slice(user_operation.call_data.as_ref()),
                call_gas_limit: user_operation.call_gas_limit.as_u64(),
                verification_gas_limit: user_operation.verification_gas_limit.as_u64(),
                pre_verification_gas: user_operation.pre_verification_gas.as_u64(),
                max_fee_per_gas: user_operation.max_fee_per_gas.as_u64(),
                max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas.as_u64(),
                paymaster_and_data: prost::bytes::Bytes::copy_from_slice(
                    user_operation.paymaster_and_data.as_ref(),
                ),
                signature: prost::bytes::Bytes::copy_from_slice(user_operation.signature.as_ref()),
            }
        }
    }

    impl From<UserOperation> for crate::types::user_operation::UserOperation {
        fn from(user_operation: UserOperation) -> Self {
            Self {
                sender: {
                    if let Some(sender) = user_operation.sender {
                        sender.into()
                    } else {
                        Address::zero()
                    }
                },
                nonce: U256::from(user_operation.nonce),
                init_code: Bytes::from(user_operation.init_code),
                call_data: Bytes::from(user_operation.call_data),
                call_gas_limit: U256::from(user_operation.call_gas_limit),
                verification_gas_limit: U256::from(user_operation.verification_gas_limit),
                pre_verification_gas: U256::from(user_operation.pre_verification_gas),
                max_fee_per_gas: U256::from(user_operation.max_fee_per_gas),
                max_priority_fee_per_gas: U256::from(user_operation.max_priority_fee_per_gas),
                paymaster_and_data: Bytes::from(user_operation.paymaster_and_data),
                signature: Bytes::from(user_operation.signature),
            }
        }
    }

    impl From<crate::types::reputation::ReputationEntry> for ReputationEntry {
        fn from(reputation_entry: crate::types::reputation::ReputationEntry) -> Self {
            Self {
                address: Some(reputation_entry.address.into()),
                uo_seen: reputation_entry.uo_seen,
                uo_included: reputation_entry.uo_included,
                status: match reputation_entry.status {
                    crate::types::reputation::ReputationStatus::OK => ReputationStatus::Ok,
                    crate::types::reputation::ReputationStatus::THROTTLED => {
                        ReputationStatus::Throttled
                    }
                    crate::types::reputation::ReputationStatus::BANNED => ReputationStatus::Banned,
                } as i32,
            }
        }
    }

    impl From<ReputationEntry> for crate::types::reputation::ReputationEntry {
        fn from(reputation_entry: ReputationEntry) -> Self {
            Self {
                address: {
                    if let Some(address) = reputation_entry.address {
                        address.into()
                    } else {
                        Address::zero()
                    }
                },
                uo_seen: reputation_entry.uo_seen,
                uo_included: reputation_entry.uo_included,
                status: match reputation_entry.status {
                    _ if reputation_entry.status == ReputationStatus::Ok as i32 => {
                        crate::types::reputation::ReputationStatus::OK
                    }
                    _ if reputation_entry.status == ReputationStatus::Throttled as i32 => {
                        crate::types::reputation::ReputationStatus::THROTTLED
                    }
                    _ if reputation_entry.status == ReputationStatus::Banned as i32 => {
                        crate::types::reputation::ReputationStatus::BANNED
                    }
                    _ => crate::types::reputation::ReputationStatus::OK,
                },
            }
        }
    }
}

pub mod uopool {
    tonic::include_proto!("uopool");
}

pub mod bundler {
    tonic::include_proto!("bundler");
}
