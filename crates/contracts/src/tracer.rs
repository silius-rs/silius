use anyhow::format_err;
use ethers::types::{Address, Bytes, GethTrace, U256};
use serde::Deserialize;
use std::collections::HashMap;

/// Object (frame) return the JavaScript tracer when simulating validation of user operation
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct JsTracerFrame {
    #[serde(rename = "numberLevels")]
    pub number_levels: Vec<Level>,
    pub keccak: Vec<Bytes>,
    pub logs: Vec<Log>,
    pub calls: Vec<Call>,
    pub debug: Vec<String>,
}

impl TryFrom<GethTrace> for JsTracerFrame {
    type Error = anyhow::Error;
    fn try_from(val: GethTrace) -> Result<Self, Self::Error> {
        match val {
            GethTrace::Known(val) => Err(format_err!("Invalid geth trace: {val:?}")),
            GethTrace::Unknown(val) => serde_json::from_value(val)
                .map_err(|error| format_err!("Failed to parse geth trace: {error}")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Level {
    pub access: HashMap<Address, ReadsAndWrites>,
    pub opcodes: HashMap<String, u64>,
    #[serde(rename = "contractSize")]
    pub contract_size: HashMap<Address, ContractSizeInfo>,
    pub oog: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ReadsAndWrites {
    pub reads: HashMap<String, u64>,
    pub writes: HashMap<String, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ContractSizeInfo {
    pub opcode: String,
    #[serde(rename = "contractSize")]
    pub contract_size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Log {
    pub topics: Vec<String>,
    pub data: Bytes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Call {
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: Option<u64>,
    pub data: Option<Bytes>,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub method: Option<Bytes>,
    pub gas: Option<u64>,
    pub value: Option<U256>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CallEntry {
    pub typ: String,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub method: Option<String>,
    pub ret: Option<Bytes>,
    pub rev: Option<Bytes>,
    pub value: Option<U256>,
}

// https://github.com/eth-infinitism/bundler/blob/main/packages/bundler/src/BundlerCollectorTracer.ts
pub const JS_TRACER: &str = r#"
{
    numberLevels: [],
    currentLevel: null,
    keccak: [],
    calls: [],
    logs: [],
    debug: [],
    lastOp: '',
    numberCounter: 0,
    fault(log, db) {
        this.debug.push(`fault depth=${log.getDepth()} gas=${log.getGas()} cost=${log.getCost()} err=${log.getError()}`);
    },
    result(ctx, db) {
        return {
            numberLevels: this.numberLevels,
            keccak: this.keccak,
            logs: this.logs,
            calls: this.calls,
            debug: this.debug // for internal debugging.
        };
    },
    enter(frame) {
        this.debug.push(`enter gas=${frame.getGas()} type=${frame.getType()} to=${toHex(frame.getTo())} in=${toHex(frame.getInput()).slice(0, 500)}`);
        this.calls.push({
            type: frame.getType(),
            from: toHex(frame.getFrom()),
            to: toHex(frame.getTo()),
            method: toHex(frame.getInput()).slice(0, 10),
            gas: frame.getGas(),
            value: frame.getValue()
        });
    },
    exit(frame) {
        this.calls.push({
            type: frame.getError() != null ? 'REVERT' : 'RETURN',
            gasUsed: frame.getGasUsed(),
            data: toHex(frame.getOutput()).slice(0, 1000)
        });
    },
    // increment the "key" in the list. if the key is not defined yet, then set it to "1"
    countSlot(list, key) {
        var _a;
        list[key] = ((_a = list[key]) !== null && _a !== void 0 ? _a : 0) + 1;
    },
    step(log, db) {
        const opcode = log.op.toString();
        // this.debug.push(this.lastOp + '- opcode + '- log.getDepth() + '- log.getGas() + '- log.getCost())
        if (log.getGas() < log.getCost()) {
            this.currentLevel.oog = true;
        }
        if (opcode === 'REVERT' || opcode === 'RETURN') {
            if (log.getDepth() === 1) {
                // exit() is not called on top-level return/revent, so we reconstruct it
                // from opcode
                const ofs = parseInt(log.stack.peek(0).toString());
                const len = parseInt(log.stack.peek(1).toString());
                const data = toHex(log.memory.slice(ofs, ofs + len)).slice(0, 1000);
                this.debug.push(opcode + ' ' + data);
                this.calls.push({
                    type: opcode,
                    gasUsed: 0,
                    data
                });
            }
        }
        if (log.getDepth() === 1) {
            // NUMBER opcode at top level split levels
            if (opcode === 'NUMBER')
                this.numberCounter++;
            if (this.numberLevels[this.numberCounter] == null) {
                this.currentLevel = this.numberLevels[this.numberCounter] = {
                    access: {},
                    opcodes: {},
                    contractSize: {}
                };
            }
            this.lastOp = '';
            return;
        }
        if (this.lastOp === 'GAS' && !opcode.includes('CALL')) {
            // count "GAS" opcode only if not followed by "CALL"
            this.countSlot(this.currentLevel.opcodes, 'GAS');
        }
        if (opcode !== 'GAS') {
            // ignore "unimportant" opcodes:
            if (opcode.match(/^(DUP\\d+|PUSH\\d+|SWAP\\d+|POP|ADD|SUB|MUL|DIV|EQ|LTE?|S?GTE?|SLT|SH[LR]|AND|OR|NOT|ISZERO)$/) == null) {
                this.countSlot(this.currentLevel.opcodes, opcode);
            }
        }
        if (opcode.match(/^(EXT.*|CALL|CALLCODE|DELEGATECALL|STATICCALL|CREATE2)$/) != null) {
            // this.debug.push('op= opcode + ' last= this.lastOp + ' stacksize= log.stack.length())
            const idx = opcode.startsWith('EXT') ? 0 : 1;
            const addr = toAddress(log.stack.peek(idx).toString(16));
            const addrHex = toHex(addr);
            // this.debug.push('op=' + opcode + ' last=' + this.lastOp + ' stacksize=' + log.stack.length() + ' addr=' + addrHex)
            if (this.currentLevel.contractSize[addrHex] == null && !isPrecompiled(addr)) {
                this.currentLevel.contractSize[addrHex] = {
                    contractSize: db.getCode(addr).length,
                    opcode
                }
            }
        }
        this.lastOp = opcode;
        if (opcode === 'SLOAD' || opcode === 'SSTORE') {
            const slot = log.stack.peek(0).toString(16);
            const addr = toHex(log.contract.getAddress());
            let access;
            if ((access = this.currentLevel.access[addr]) == null) {
                this.currentLevel.access[addr] = access = {
                    reads: {},
                    writes: {}
                };
            }
            this.countSlot(opcode === 'SLOAD' ? access.reads : access.writes, slot);
        }
        if (opcode === 'KECCAK256') {
            // collect keccak on 64-byte blocks
            const ofs = parseInt(log.stack.peek(0).toString());
            const len = parseInt(log.stack.peek(1).toString());
            // currently, solidity uses only 2-word (6-byte) for a key. this might change..
            // still, no need to return too much
            if (len > 20 && len < 512) {
                // if (len === 64) {
                this.keccak.push(toHex(log.memory.slice(ofs, ofs + len)));
            }
        }
        else if (opcode.startsWith('LOG')) {
            const count = parseInt(opcode.substring(3));
            const ofs = parseInt(log.stack.peek(0).toString());
            const len = parseInt(log.stack.peek(1).toString());
            const topics = [];
            for (let i = 0; i < count; i++) {
                // eslint-disable-next-line @typescript-eslint/restrict-plus-operands
                topics.push('0x' + log.stack.peek(2 + i).toString(16));
            }
            const data = toHex(log.memory.slice(ofs, ofs + len));
            this.logs.push({
                topics,
                data
            });
        }
    }
}
"#;
