// Copyright (c) 2018-2022 The MobileCoin Foundation

//! TODO

use super::EthAddr;
use mc_util_serial::JsonU64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
