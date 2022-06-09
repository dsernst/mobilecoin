// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Gnosis Safe transaction fetcher, used to get the transaction data from a
//! gnosis safe-transaction-service.
//!
//! TODO
//! - figure out what to return from get_all_transactions so that we capture
//!   both the full response, and optionally the decoded transaction data
//! - need to include offsets
//! - need to store in lmdb: 1) map of real tx hash -> data (used to lookup from
//!   mint nonce) 2) list of all hashes in chronological order? reverse
//!   chronological order? lmdb ordering is messsy
//! - code that takes a MintTx and returns the matching
//!   DecodedGnosisSafeTransaction, this needs to: 1) lookup by the nonce 2)
//!   compare the amount
//! - code that takes DecodedGnosisSafeTransaction and if it contains a burn
//!   (moving token out + burn memo multi-tx) try and locate matching mc
//!   transaction (lookup by txout pub key)
//!
//! two scanning modes:
//! 1) everything
//! 2) until reaching a known hash

use super::{api_data_types, error::Error};
use super::EthAddr;
use mc_common::logger::{log, o, Logger};
use reqwest::{blocking::Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use url::Url;

pub type EthTxHash = String;

// TODO move somewhere else
#[derive(Debug, Deserialize, Serialize)]
pub struct GnosisSafeTransaction {
    raw: Value,
}

impl GnosisSafeTransaction {
    pub fn decode(&self) -> Result<api_data_types::Transaction, Error> {
        Ok(serde_json::from_value(self.raw.clone())?)
    }

    pub fn tx_hash(&self) -> Result<EthTxHash, Error> {
        let hash_str = self
            .raw
            .get("transactionHash")
            .or_else(|| self.raw.get("txHash"))
            .and_then(|val| val.as_str())
            .ok_or(Error::Other(
                "GnosisSafeTransaction: missing transactionHash".to_string(),
            ))?;
        EthTxHash::from_str(hash_str)
            .map_err(|err| Error::Other(format!("Failed parsing tx hash '{}': {}", hash_str, err)))
    }

    pub fn to_json_string(&self) -> String {
        self.raw.to_string()
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self::from(serde_json::from_slice::<Value>(bytes)?))
    }
}

impl From<Value> for GnosisSafeTransaction {
    fn from(value: Value) -> Self {
        Self { raw: value }
    }
}

/// Gnosis Safe transaction fetcher, used to get the transaction data from a
/// gnosis safe-transaction-service.
pub struct GnosisSafeFetcher {
    /// Base URL for the gnosis safe-transaction-service API.
    base_url: Url,

    /// The [reqwest::Client].
    client: Client,

    /// Logger.
    logger: Logger,
}

impl GnosisSafeFetcher {
    /// Instantiate a [GnosisSafeFetcher] fetching transactions from the given
    /// URL endpoint.
    /// The URL endpoint is expected to run the Gnosis safe-transaction-service
    /// (https://github.com/safe-global/safe-transaction-service/)
    pub fn new(mut base_url: Url, logger: Logger) -> Result<Self, Error> {
        if !base_url.path().ends_with('/') {
            base_url = base_url.join(&format!("{}/", base_url.path()))?;
        }

        let logger = logger.new(o!("url" => base_url.to_string()));

        let client = Client::builder()
            .build()
            .map_err(|e| Error::Other(format!("Failed to create reqwest client: {}", e)))?;

        Ok(Self {
            base_url,
            client,
            logger,
        })
    }

    /// Fetch transaction data.
    /// This returns only transactions that were executed and confirmed.
    pub fn get_transaction_data(
        &self,
        safe_address: &EthAddr,
    ) -> Result<Vec<GnosisSafeTransaction>, Error> {
        let url = self.base_url.join(&format!(
            "api/v1/safes/{}/all-transactions/?executed=true&queued=false&trusted=true",
            safe_address
        ))?;
        log::debug!(self.logger, "Fetching transactions from: {}", url);

        let response = self
            .client
            .get(url.as_str())
            .send()
            .map_err(|err| Error::Other(format!("Failed to fetch '{}': {}", url, err)))?;
        if response.status() != StatusCode::OK {
            return Err(Error::Other(format!(
                "Failed to fetch '{}': Expected status 200, got {}",
                url,
                response.status()
            )));
        }

        let data = response
            .json::<api_data_types::AllTransactionsResponse>()
            .map_err(|err| Error::Other(format!("Failed parsing JSON from '{}': {}", url, err)))?;

        Ok(data
            .results
            .into_iter()
            .map(GnosisSafeTransaction::from)
            .collect())
    }
}
