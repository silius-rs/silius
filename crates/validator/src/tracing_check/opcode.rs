use alloy_primitives::{Address, U160};
use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use revm_bytecode::OpCode;
use silius_chain::Chain;
use silius_primitives::{network_spec::network_spec, user_operation::UserOperation};
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    tracing_check::{TracingCheck, TracingContext, error::TracingCheckError},
};

pub const BLOCKED_OPCODES: [OpCode; 13] = [
    OpCode::ORIGIN,
    OpCode::GASPRICE,
    OpCode::BLOCKHASH,
    OpCode::COINBASE,
    OpCode::TIMESTAMP,
    OpCode::NUMBER,
    OpCode::DIFFICULTY,
    OpCode::GASLIMIT,
    OpCode::BASEFEE,
    OpCode::BLOBHASH,
    OpCode::BLOBBASEFEE,
    OpCode::INVALID,
    OpCode::SELFDESTRUCT,
];

pub struct OpcodeCheck;

#[async_trait::async_trait]
impl<P: Provider> TracingCheck<P> for OpcodeCheck {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        _config: &ValidatorConfig,
        _db: &SiliusDB,
        _chain: &Chain<P>,
        frame: &Erc7562Frame,
        context: &TracingContext,
    ) -> Result<(), TracingCheckError> {
        if let Some(to) = frame.to
            && to != network_spec().entry_point_address
        {
            for opcode in frame.used_opcodes.keys() {
                let opcode = OpCode::new(*opcode).unwrap_or_default();
                if BLOCKED_OPCODES.contains(&opcode) {
                    return Err(TracingCheckError::BannedOpcode(opcode.to_string()));
                }
            }
        }

        for (address, contract_size) in frame.contract_size.iter() {
            let address = address.clone();

            if _is_precompile(&address) {
                continue;
            }

            if address != user_operation.sender
                && address != network_spec().entry_point_address
                && contract_size.contract_size <= 2
            {
                return Err(TracingCheckError::UndeployedContractAccess(
                    context.current_entity,
                    address,
                    OpCode::new(contract_size.opcode)
                        .unwrap_or_default()
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

fn _is_precompile(address: &Address) -> bool {
    let address: U160 = (*address).into();
    address >= U160::from(1) && address < U160::from(1000)
}
