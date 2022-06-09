// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Gnosis Safe transaction fetcher, used to get the transaction data from a
//! gnosis safe-transaction-service.
//!
//! TODO
//! - need to include offsets
//! two scanning modes:
//! 1) everything
//! 2) until reaching a known hash

use super::{
    api_data_types::{AllTransactionsResponse, RawGnosisTransaction},
    Error, EthAddr,
};
use mc_common::logger::{log, o, Logger};
use reqwest::{blocking::Client, StatusCode};
use url::Url;

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
    ) -> Result<Vec<RawGnosisTransaction>, Error> {
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
            .json::<AllTransactionsResponse>()
            .map_err(|err| Error::Other(format!("Failed parsing JSON from '{}': {}", url, err)))?;

        Ok(data
            .results
            .into_iter()
            .map(RawGnosisTransaction::from)
            .collect())
    }
}
