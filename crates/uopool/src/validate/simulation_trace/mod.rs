//! `SimulationTrace` module performs checks against a [UserOperation's](UserOperation) call stack,
//! code hashes, external contract access, gas, opcodes, and storage access by initiating a
//! `debug_traceCall` to a Ethereum execution client.
pub mod call_stack;
pub mod code_hashes;
pub mod external_contracts;
pub mod gas;
pub mod opcodes;
pub mod storage_access;
