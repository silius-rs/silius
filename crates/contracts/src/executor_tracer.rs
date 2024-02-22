use ethers::types::GethTrace;
use eyre::format_err;
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct LogInfo {
    pub topics: Vec<String>,
    pub data: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ExecutorTracerResult {
    pub reverts: Vec<String>,
    #[serde(rename = "validationOOG")]
    pub validation_oog: bool,
    #[serde(rename = "executionOOG")]
    pub execution_oog: bool,
    #[serde(rename = "executionGasLimit")]
    pub execution_gas_limit: u64,
    #[serde(rename = "userOperationEvent")]
    pub user_op_event: Option<LogInfo>,
    #[serde(rename = "userOperationRevertEvent")]
    pub user_op_revert_event: Option<LogInfo>,
    pub output: String,
    pub error: String,
}
impl TryFrom<GethTrace> for ExecutorTracerResult {
    type Error = eyre::Error;
    fn try_from(val: GethTrace) -> Result<Self, Self::Error> {
        match val {
            GethTrace::Known(val) => Err(format_err!("Invalid geth trace: {val:?}")),
            GethTrace::Unknown(val) => serde_json::from_value(val.clone())
                .map_err(|error| format_err!("Failed to parse geth trace: {error}, {val:#}")),
        }
    }
}
pub const EXECUTOR_TRACER: &str = r#"
{
    reverts: [],
    validationOOG: false,
    executionOOG: false,
    executionGasLimit: 0,
  
    _depth: 0,
    _executionGasStack: [],
    _defaultGasItem: { used: 0, required: 0 },
    _marker: 0,
    _validationMarker: 1,
    _executionMarker: 3,
    _userOperationEventTopics0:
      "0x49628fd1471006c1482da88028e9ce4dbb080b815c9b0344d39e5a8e6ec1419f",
    _userOperationRevertEventTopics0:
      "0x1c4fada7374c0a9ee8841fc38afe82932dc0f8e69012e927f061a8bae611a201",
  
    _isValidation: function () {
      return (
        this._marker >= this._validationMarker &&
        this._marker < this._executionMarker
      );
    },
  
    _isExecution: function () {
      return this._marker === this._executionMarker;
    },
  
    _isUserOperationEvent: function (log) {
      var topics0 = "0x" + log.stack.peek(2).toString(16);
      return topics0 === this._userOperationEventTopics0;
    },
  
    _setUserOperationEvent: function (opcode, log) {
      var count = parseInt(opcode.substring(3));
      var ofs = parseInt(log.stack.peek(0).toString());
      var len = parseInt(log.stack.peek(1).toString());
      var topics = [];
      for (var i = 0; i < count; i++) {
        topics.push(log.stack.peek(2 + i).toString(16));
      }
      var data = toHex(log.memory.slice(ofs, ofs + len));
      this.userOperationEvent = {
        topics: topics,
        data: data,
      };
    },
  
    _isUserOperationRevertEvent: function (log) {
      var topics0 = "0x" + log.stack.peek(2).toString(16);
      return topics0 === this._userOperationRevertEventTopics0;
    },
  
    _setUserOperationRevertEvent: function (opcode, log) {
      var count = parseInt(opcode.substring(3));
      var ofs = parseInt(log.stack.peek(0).toString());
      var len = parseInt(log.stack.peek(1).toString());
      var topics = [];
      for (var i = 0; i < count; i++) {
        topics.push(log.stack.peek(2 + i).toString(16));
      }
      var data = toHex(log.memory.slice(ofs, ofs + len));
      this.userOperationRevertEvent = {
        topics: topics,
        data: data,
      };
    },
    fault: function fault(log, db) {},
    result: function result(ctx, db) {
      return {
        reverts: this.reverts,
        validationOOG: this.validationOOG,
        executionOOG: this.executionOOG,
        executionGasLimit: this.executionGasLimit,
        userOperationEvent: this.userOperationEvent,
        userOperationRevertEvent: this.userOperationRevertEvent,
        output: toHex(ctx.output),
        error: ctx.error,
      };
    },
  
    enter: function enter(frame) {
      if (this._isExecution()) {
        var next = this._depth + 1;
        if (this._executionGasStack[next] === undefined)
          this._executionGasStack[next] = Object.assign({}, this._defaultGasItem);
      }
    },
    exit: function exit(frame) {
      if (this._isExecution()) {
        if (frame.getError() !== undefined) {
          this.reverts.push(toHex(frame.getOutput()));
        }
  
        if (this._depth >= 2) {
          // Get the final gas item for the nested frame.
          var nested = Object.assign(
            {},
            this._executionGasStack[this._depth + 1] || this._defaultGasItem
          );
  
          // Reset the nested gas item to prevent double counting on re-entry.
          this._executionGasStack[this._depth + 1] = Object.assign(
            {},
            this._defaultGasItem
          );
  
          // Keep track of the total gas used by all frames at this depth.
          // This does not account for the gas required due to the 63/64 rule.
          var used = frame.getGasUsed();
          this._executionGasStack[this._depth].used += used;
  
          // Keep track of the total gas required by all frames at this depth.
          // This accounts for additional gas needed due to the 63/64 rule.
          this._executionGasStack[this._depth].required +=
            used - nested.used + Math.ceil((nested.required * 64) / 63);
  
          // Keep track of the final gas limit.
          this.executionGasLimit = this._executionGasStack[this._depth].required;
        }
      }
    },
  
    step: function step(log, db) {
      var opcode = log.op.toString();
      this._depth = log.getDepth();
      if (this._depth === 1 && opcode === "NUMBER") this._marker++;
  
      if (
        this._depth <= 2 &&
        opcode.startsWith("LOG") &&
        this._isUserOperationEvent(log)
      )
        this._setUserOperationEvent(opcode, log);
      if (
          this._depth <= 2 &&
          opcode.startsWith("LOG") &&
          this._isUserOperationRevertEvent(log)
        )
          this._setUserOperationRevertEvent(opcode, log);
    
      if (log.getGas() < log.getCost() && this._isValidation())
        this.validationOOG = true;
  
      if (log.getGas() < log.getCost() && this._isExecution())
        this.executionOOG = true;
    },
  } 
"#;

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    // Json Test for the `ExecutorTracerResult` struct
    #[test]
    fn test_json() {
        #[derive(Serialize, Deserialize, Debug)]
        struct A {
            data: Vec<u8>,
        }
        let data = r#"
        {
            "data": [0,0,195,0,0]
        }"#;
        let v: Value = serde_json::from_str(data).unwrap();
        println!("{:?}", v);
        let a: A = serde_json::from_value(v).unwrap();
        println!("{:?}", a);
    }
}
