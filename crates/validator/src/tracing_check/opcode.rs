use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use revm_bytecode::OpCode;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    tracing_check::{TracingCheck, error::TracingCheckError},
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
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
        frame: &Erc7562Frame,
    ) -> Result<(), TracingCheckError> {
        for opcode in frame.used_opcodes.keys() {
            let opcode = OpCode::new(*opcode).unwrap_or_default();
            if BLOCKED_OPCODES.contains(&opcode) {
                return Err(TracingCheckError::BannedOpcode(opcode.to_string()));
            }
        }

        Ok(())
    }
}
