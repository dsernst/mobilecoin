// Copyright (c) 2018-2022 The MobileCoin Foundation

//! TODO

use super::EthAddr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Represents u64 using string, when serializing to Json
// Javascript integers are not 64 bit, and so it is not really proper json.
// Using string avoids issues with some json parsers not handling large numbers
// well.
//
// This does not rely on the serde-json arbitrary precision feature, which
// (we fear) might break other things (e.g. https://github.com/serde-rs/json/issues/505)
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct JsonU64(#[serde(with = "serde_with::rust::display_fromstr")] pub u64);

impl From<&u64> for JsonU64 {
    fn from(src: &u64) -> Self {
        Self(*src)
    }
}

impl From<&JsonU64> for u64 {
    fn from(src: &JsonU64) -> u64 {
        src.0
    }
}

impl From<JsonU64> for u64 {
    fn from(src: JsonU64) -> u64 {
        src.0
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AllTransactionsResponse {
    pub count: u64,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataDecoded {
    pub method: String,
    pub parameters: Vec<DataDecodedParameter>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataDecodedParameter {
    pub name: String,
    // type: String,
    pub value: String,

    #[serde(rename = "valueDecoded")]
    pub value_decoded: Option<Vec<ValueDecoded>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValueDecoded {
    pub operation: u64,
    pub to: EthAddr,
    pub value: String,
    pub data: String,

    #[serde(rename = "dataDecoded")]
    pub data_decoded: Option<Box<DataDecoded>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultiSigTransaction {
    pub safe: EthAddr,
    pub to: EthAddr,
    pub value: String,
    pub data: Option<String>,
    #[serde(rename = "transactionHash")]
    pub tx_hash: String,

    #[serde(rename = "dataDecoded")]
    pub data_decoded: Option<DataDecoded>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EthereumTransfer {
    pub from: EthAddr,
    pub to: EthAddr,

    /// None for Eth transfers
    #[serde(rename = "tokenAddress")]
    pub token_address: Option<EthAddr>,

    #[serde(rename = "transactionHash")]
    pub tx_hash: String,

    #[serde(rename = "type")]
    pub tx_type: String,

    pub value: JsonU64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EthereumTransaction {
    #[serde(rename = "txHash")]
    pub tx_hash: String,

    pub transfers: Vec<EthereumTransfer>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "txType")]
pub enum Transaction {
    #[serde(rename = "MULTISIG_TRANSACTION")]
    MultiSig(MultiSigTransaction),

    #[serde(rename = "ETHEREUM_TRANSACTION")]
    Ethereum(EthereumTransaction),

    #[serde(rename = "MODULE_TRANSACTION")]
    Module(Value),
}
