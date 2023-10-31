use ethers::types::{Address, Bytes, GethTrace, U256};
use eyre::format_err;
use serde::Deserialize;
use std::collections::HashMap;

/// Object (frame) return the JavaScript tracer when simulating validation of user operation
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct JsTracerFrame {
    #[serde(rename = "callsFromEntryPoint")]
    pub calls_from_entry_point: Vec<TopLevelCallInfo>,
    pub keccak: Vec<Bytes>,
    pub logs: Vec<Log>,
    pub calls: Vec<Call>,
    pub debug: Vec<String>,
}

impl TryFrom<GethTrace> for JsTracerFrame {
    type Error = eyre::Error;
    fn try_from(val: GethTrace) -> Result<Self, Self::Error> {
        match val {
            GethTrace::Known(val) => Err(format_err!("Invalid geth trace: {val:?}")),
            GethTrace::Unknown(val) => serde_json::from_value(val.clone())
                .map_err(|error| format_err!("Failed to parse geth trace: {error}, {val:#}")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct TopLevelCallInfo {
    #[serde(rename = "topLevelMethodSig")]
    pub top_level_method_sig: Bytes,
    #[serde(rename = "topLevelTargetAddress")]
    pub top_level_target_address: Bytes,
    pub access: HashMap<Address, ReadsAndWrites>,
    pub opcodes: HashMap<String, u64>,
    #[serde(rename = "contractSize")]
    pub contract_size: HashMap<Address, ContractSizeInfo>,
    #[serde(rename = "extCodeAccessInfo")]
    pub ext_code_access_info: HashMap<Address, String>,
    pub oog: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ReadsAndWrites {
    pub reads: HashMap<String, String>,
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
    callsFromEntryPoint: [],
    currentLevel: null,
    keccak: [],
    calls: [],
    logs: [],
    debug: [],
    lastOp: '',
    lastThreeOpcodes: [],
    // event sent after all validations are done: keccak("BeforeExecution()")
    stopCollectingTopic: 'bb47ee3e183a558b1a2ff0874b079f3fc5478b7454eacf2bfc5af2ff5878f972',
    stopCollecting: false,
    topLevelCallCounter: 0,

    fault: function(log, db) {
        this.debug.push(`fault depth=${log.getDepth()} gas=${log.getGas()} cost=${log.getCost()} err=${log.getError()}`);
    },

    result: function(ctx, db) {
        return {
            callsFromEntryPoint: this.callsFromEntryPoint,
            keccak: this.keccak,
            logs: this.logs,
            calls: this.calls,
            debug: this.debug // for internal debugging.
        };
    },

    enter: function(frame) {
        if (this.stopCollecting) {
            return;
        }
        // this.debug.push('enter gas=', frame.getGas(), ' type=', frame.getType(), ' to=', toHex(frame.getTo()), ' in=', toHex(frame.getInput()).slice(0, 500))
        this.calls.push({
            type: frame.getType(),
            from: toHex(frame.getFrom()),
            to: toHex(frame.getTo()),
            method: toHex(frame.getInput()).slice(0, 10),
            gas: frame.getGas(),
            value: frame.getValue()
        });
    },

    exit: function(frame) {
        if (this.stopCollecting) {
            return;
        }
        this.calls.push({
            type: frame.getError() != null ? 'REVERT' : 'RETURN',
            gasUsed: frame.getGasUsed(),
            data: toHex(frame.getOutput()).slice(0, 4000)
        });
    },

    // increment the "key" in the list. if the key is not defined yet, then set it to "1"
    countSlot: function(list, key) {
        var _a;
        list[key] = ((_a = list[key]) !== null && _a !== void 0 ? _a : 0) + 1;
    },

    step: function(log, db) {
        var _a;
        if (this.stopCollecting) {
            return;
        }
        const opcode = log.op.toString();
        const stackSize = log.stack.length();
        const stackTop3 = [];
        for (let i = 0; i < 3 && i < stackSize; i++) {
            stackTop3.push(log.stack.peek(i));
        }
        this.lastThreeOpcodes.push({
            opcode: opcode,
            stackTop3: stackTop3
        });
        if (this.lastThreeOpcodes.length > 3) {
            this.lastThreeOpcodes.shift();
        }
        // this.debug.push(this.lastOp + '-' + opcode + '-' + log.getDepth() + '-' + log.getGas() + '-' + log.getCost())
        if (log.getGas() < log.getCost() || (
            // special rule for SSTORE with gas metering
            opcode === 'SSTORE' && log.getGas() < 2300)
        ) {
            this.currentLevel.oog = true;
        }
        if (opcode === 'REVERT' || opcode === 'RETURN') {
            if (log.getDepth() === 1) {
                // exit() is not called on top-level return/revent, so we reconstruct it
                // from opcode
                const ofs = parseInt(log.stack.peek(0).toString());
                const len = parseInt(log.stack.peek(1).toString());
                const data = toHex(log.memory.slice(ofs, ofs + len)).slice(0, 4000);
                // this.debug.push(opcode + ' ' + data)
                this.calls.push({
                    type: opcode,
                    gasUsed: 0,
                    data: data
                });
            }
            // NOTE: flushing all history after RETURN
            this.lastThreeOpcodes = [];
        }
        if (log.getDepth() === 1) {
            if (opcode === 'CALL' || opcode === 'STATICCALL') {
                // stack.peek(0) - gas
                const addr = toAddress(log.stack.peek(1).toString(16));
                const topLevelTargetAddress = toHex(addr);
                // stack.peek(2) - value
                const ofs = parseInt(log.stack.peek(3).toString());
                // stack.peek(4) - len
                const topLevelMethodSig = toHex(log.memory.slice(ofs, ofs + 4));
                this.currentLevel = this.callsFromEntryPoint[this.topLevelCallCounter] = {
                    topLevelMethodSig: topLevelMethodSig,
                    topLevelTargetAddress: topLevelTargetAddress,
                    access: {},
                    opcodes: {},
                    extCodeAccessInfo: {},
                    contractSize: {}
                };
                this.topLevelCallCounter++;
            } else if (opcode === 'LOG1') {
                // ignore log data ofs, len
                const topic = log.stack.peek(2).toString(16);
                if (topic === this.stopCollectingTopic) {
                    this.stopCollecting = true;
                }
            }
            this.lastOp = '';
            return;
        }
        const lastOpInfo = this.lastThreeOpcodes[this.lastThreeOpcodes.length - 2];
        // store all addresses touched by EXTCODE* opcodes
        if (((_a = lastOpInfo === null || lastOpInfo === void 0 ? void 0 : lastOpInfo.opcode) === null || _a === void 0 ? void 0 : _a.match(/^(EXT.*)$/)) != null) {
            const addr = toAddress(lastOpInfo.stackTop3[0].toString(16));
            const addrHex = toHex(addr);
            const last3opcodesString = this.lastThreeOpcodes.map(function(x) {
                return x.opcode;
            }).join(' ');
            // only store the last EXTCODE* opcode per address - could even be a boolean for our current use-case
            // [OP-051] - may call EXTCODESIZE ISZERO
            if (last3opcodesString.match(/^(\w+) EXTCODESIZE ISZERO$/) == null) {
                this.currentLevel.extCodeAccessInfo[addrHex] = opcode;
                // this.debug.push(`potentially illegal EXTCODESIZE without ISZERO for ${addrHex}`)
            } else {
                // this.debug.push(`safe EXTCODESIZE with ISZERO for ${addrHex}`)
            }
        }
        // not using 'isPrecompiled' to only allow the ones defined by the ERC-4337 as stateless precompiles
        // [OP-062] - only allowed the core 9 precompiles
        const isAllowedPrecompiled = function(address) {
            const addrHex = toHex(address);
            const addressInt = parseInt(addrHex);
            // this.debug.push(`isPrecompiled address=${addrHex} addressInt=${addressInt}`)
            return addressInt > 0 && addressInt < 10;
        };
        // [OP-041] - access to an address without a deployed code is forbidden for EXTCODE* and *CALL opcodes
        if (opcode.match(/^(EXT.*|CALL|CALLCODE|DELEGATECALL|STATICCALL)$/) != null) {
            const idx = opcode.startsWith('EXT') ? 0 : 1;
            const addr = toAddress(log.stack.peek(idx).toString(16));
            const addrHex = toHex(addr);
            // this.debug.push('op=' + opcode + ' last=' + this.lastOp + ' stacksize=' + log.stack.length() + ' addr=' + addrHex)
            if (this.currentLevel.contractSize[addrHex] == null && !isAllowedPrecompiled(addr)) {
                this.currentLevel.contractSize[addrHex] = {
                    contractSize: db.getCode(addr).length,
                    opcode: opcode
                };
            }
        }
        // [OP-012] - GAS opcode is allowed, but only if followed immediately by *CALL instructions
        if (this.lastOp === 'GAS' && !opcode.includes('CALL')) {
            // count "GAS" opcode only if not followed by "CALL"
            this.countSlot(this.currentLevel.opcodes, 'GAS');
        }
        if (opcode !== 'GAS') {
            // ignore "unimportant" opcodes:
            if (opcode.match(/^(DUP\d+|PUSH\d+|SWAP\d+|POP|ADD|SUB|MUL|DIV|EQ|LTE?|S?GTE?|SLT|SH[LR]|AND|OR|NOT|ISZERO)$/) == null) {
                this.countSlot(this.currentLevel.opcodes, opcode);
            }
        }
        this.lastOp = opcode;
        if (opcode === 'SLOAD' || opcode === 'SSTORE') {
            const slot = toWord(log.stack.peek(0).toString(16));
            const slotHex = toHex(slot);
            const addr = log.contract.getAddress();
            const addrHex = toHex(addr);
            let access = this.currentLevel.access[addrHex];
            if (access == null) {
                access = {
                    reads: {},
                    writes: {}
                };
                this.currentLevel.access[addrHex] = access;
            }
            if (opcode === 'SLOAD') {
                // read slot values before this UserOp was created
                // (so saving it if it was written before the first read)
                if (access.reads[slotHex] == null && access.writes[slotHex] == null) {
                    access.reads[slotHex] = toHex(db.getState(addr, slot));
                }
            } else {
                this.countSlot(access.writes, slotHex);
            }
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
        } else if (opcode.startsWith('LOG')) {
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
                topics: topics,
                data: data
            });
        }
    }
}
"#;
