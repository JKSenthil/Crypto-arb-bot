use std::collections::HashMap;

use ethers::types::Address;
use ethers::types::Bytes;
use ethers::types::H256;
use ethers::types::U256;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraceConfig {
    pub disable_storage: bool,
    pub disable_stack: bool,
    pub enable_memory: bool,
    pub enable_return_data: bool,
    pub tracer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracer_config: Option<TracerConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TracerConfig {
    pub only_top_call: bool,
    pub with_log: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Res {
    pub result: BlockTraceResult,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockTraceResult {
    pub from: Address,
    pub gas: U256,
    pub gas_used: U256,
    pub input: Bytes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Bytes>,
    pub to: Address,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<Vec<BlockTraceResult>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxpoolEntry {
    pub hash: H256,
    pub gas_price: U256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TxpoolContent {
    pub pending: HashMap<Address, HashMap<U256, TxpoolEntry>>,
    pub queued: HashMap<Address, HashMap<U256, TxpoolEntry>>,
}
