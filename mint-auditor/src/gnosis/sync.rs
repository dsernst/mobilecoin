// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Code for syncing transactions from the Gnosis API into the SQLite database.
//!
//! NOTE: Right now, if the audited safes
//! configuration changes, one should delete the SQLite database and re-audit.
//! The code is not smart enough to handle adding/removing safes/tokens for
//! Gnosis transactions that were already processed.

use super::{
    api_data_types::{EthereumTransaction, MultiSigTransaction, Transaction},
    fetcher::{GnosisSafeFetcher, GnosisSafeTransaction},
    AuditedSafeConfig,
};
use crate::{
    db::{
        transaction, Conn, GnosisSafeDeposit, GnosisSafeDepositModel, GnosisSafeTx,
        GnosisSafeTxModel, GnosisSafeWithdrawal, GnosisSafeWithdrawalModel, MintAuditorDb,
        NewGnosisSafeWithdrawal,
    },
    error::Error,
    gnosis::Error as GnosisError,
};
use mc_common::logger::{log, Logger};

/// An object for syncing transaction data from the Gnosis API into the SQLite
/// database.
pub struct GnosisSync {
    fetcher: GnosisSafeFetcher,
    audited_safe: AuditedSafeConfig,
    mint_auditor_db: MintAuditorDb,
    logger: Logger,
}

impl GnosisSync {
    pub fn new(
        audited_safe: AuditedSafeConfig,
        mint_auditor_db: MintAuditorDb,
        logger: Logger,
    ) -> Result<Self, Error> {
        Ok(Self {
            fetcher: GnosisSafeFetcher::new(audited_safe.api_url.clone(), logger.clone())?,
            audited_safe,
            mint_auditor_db,
            logger,
        })
    }
    pub fn poll(&self) {
        // TODO handle pagination (offset, limit)
        match self
            .fetcher
            .get_transaction_data(&self.audited_safe.safe_addr)
        {
            Ok(transactions) => {
                self.process_transactions(transactions);
            }
            Err(err) => {
                log::error!(self.logger, "Failed to fetch Gnosis transactions: {}", err);
            }
        }
    }

    pub fn process_transactions(&self, transactions: Vec<GnosisSafeTransaction>) {
        for tx in transactions {
            let conn = self
                .mint_auditor_db
                .get_conn()
                .expect("failed getting connection");
            transaction(&conn, |conn| -> Result<(), Error> {
                match GnosisSafeTx::insert(&tx, &conn) {
                    Ok(_) => {}
                    Err(Error::AlreadyExists(_)) => {
                        log::trace!(
                            self.logger,
                            "Skipping already-processed eth transaction {:?}",
                            tx.tx_hash()
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        log::error!(self.logger, "Failed to insert GnosisSafeTx: {}", err);
                        return Err(err);
                    }
                };

                match tx.decode()? {
                    Transaction::Ethereum(eth_tx) => {
                        self.process_eth_transaction(conn, &eth_tx)?;
                    }
                    Transaction::MultiSig(multi_sig_tx) => {
                        self.process_multi_sig_transaction(conn, &multi_sig_tx)?;
                    }
                    Transaction::Module(value) => {
                        log::warn!(
                            self.logger,
                            "Got unexpected \"Module\" transaction: {:?}",
                            value
                        );
                    }
                };

                Ok(())
            })
            .expect("failed processing transaction");
        }
    }

    /// Process an Ethereum transaction.
    fn process_eth_transaction(&self, conn: &Conn, tx: &EthereumTransaction) -> Result<(), Error> {
        log::trace!(self.logger, "Processing Ethereum transaction: {:?}", tx);

        for transfer in &tx.transfers {
            // See if this is a deposit to the safe.
            if transfer.to == self.audited_safe.safe_addr {
                log::info!(
                    self.logger,
                    "Processing gnosis safe deposit: {:?}",
                    transfer
                );
                GnosisSafeDeposit::insert_eth_transfer(transfer, &conn)?;
                continue;
            }
            // We don't know what this is.
            else {
                log::crit!(
                    self.logger,
                    "Unknown transfer {:?} in eth tx {}",
                    transfer,
                    tx.tx_hash,
                );
            }
        }

        Ok(())
    }

    /// Process a MultiSig transaction.
    fn process_multi_sig_transaction(
        &self,
        conn: &Conn,
        multi_sig_tx: &MultiSigTransaction,
    ) -> Result<(), Error> {
        match self.parse_withdrawal_with_pub_key_multi_sig_tx(multi_sig_tx) {
            Ok(withdrawal) => {
                log::info!(
                    self.logger,
                    "Processing withdrawal from multi-sig tx: {:?}",
                    withdrawal
                );

                GnosisSafeWithdrawal::insert(&withdrawal, conn)?;
            }

            Err(err) => {
                log::warn!(
                    self.logger,
                    "Failed parsing a withdrawal from multisig tx {}: {}",
                    multi_sig_tx.tx_hash,
                    err
                );
            }
        };

        Ok(())
    }

    /// See if this is a multi-sig withdrawal that uses the auxiliary contract
    /// for recording the tx out public key, and if so parse it into a
    /// [NewGnosisSafeWithdrawal] object.
    fn parse_withdrawal_with_pub_key_multi_sig_tx(
        &self,
        multi_sig_tx: &MultiSigTransaction,
    ) -> Result<NewGnosisSafeWithdrawal, GnosisError> {
        // Get the decoded data - this is the part that contains details about the
        // individual transfers included in the multi-transfer.
        let data = multi_sig_tx
            .data_decoded
            .as_ref()
            .ok_or_else(|| GnosisError::ApiResultParse("data_decoded is empty".into()))?;

        if data.method != "multiSend" {
            return Err(GnosisError::ApiResultParse(format!(
                "multi-sig tx method mismatch: got {}, expected multiSend",
                data.method
            )));
        }

        // The decoded data is expected to contain a single "transactions" parameter,
        // which should be an array of the individual transfers.
        if data.parameters.len() != 1 {
            return Err(GnosisError::ApiResultParse(format!(
                "invalid number of parameters: got {}, expected 1",
                data.parameters.len()
            )));
        }

        let parameter = &data.parameters[0];
        let value_decoded = parameter.value_decoded.as_ref().ok_or_else(|| {
            GnosisError::ApiResultParse("decoded data parameter is missing value_decoded".into())
        })?;

        // Each value contains a single transfer. We expect to have two trasnfers:
        // 1) A transfer moving the token being withdrawn from the safe
        // 2) A "dummy" transfer into the auxiliary contract, used to record the
        // matching MobileCoin tx out public key of the matching burn.
        if value_decoded.len() != 2 {
            return Err(GnosisError::ApiResultParse(format!(
                "Invalid number of values in multiSend transfer: got {}, expected 2",
                value_decoded.len()
            )));
        }

        // The first value is the transfer of the actual token held in the safe. It
        // should match a token we are auditing.
        let transfer_data = &value_decoded[0];
        let audited_token = self
            .audited_safe
            .get_token_by_eth_contract_addr(&transfer_data.to)
            .ok_or_else(|| {
                GnosisError::ApiResultParse(format!(
                    "Encountered multiSend transaction to an unknown token: {}",
                    transfer_data.to
                ))
            })?;

        // The first value (transfer of token held in safe) should contain two
        // parameters - the ethereum address receiving the withdrawal and the
        // amount being moved out of the safe.
        let transfer_data_decoded = transfer_data.data_decoded.as_ref().ok_or_else(|| {
            GnosisError::ApiResultParse("multiSend transfer first value has no decided data".into())
        })?;
        if transfer_data_decoded.method != "transfer" {
            return Err(GnosisError::ApiResultParse(format!(
                "Invalid first value method: got {}, expected transfer",
                transfer_data_decoded.method
            )));
        }

        let value_str = transfer_data_decoded
            .parameters
            .iter()
            .find_map(|param| {
                if param.name == "value" {
                    Some(&param.value)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                GnosisError::ApiResultParse("first value is missing the \"value\" parameter".into())
            })?;
        let transfer_value = value_str.parse::<u64>().map_err(|err| {
            GnosisError::ApiResultParse(format!(
                "invalid first value parameter: \"value\" {} cannot be be converted to u64: {}",
                value_str, err,
            ))
        })?;

        // The second value (dummy transfer to auxiliary contract) shoulld contain the
        // MobileCoin tx out public key in the data. There is no decoded version
        // of the data since the Gnosis API does not how to decode custom contracts.
        let aux_contract_value = &value_decoded[1];
        if aux_contract_value.to != audited_token.aux_burn_contract_addr {
            return Err(GnosisError::ApiResultParse(format!(
                "aux contract destination mismatch: got {}, expected {}",
                aux_contract_value.to, audited_token.aux_burn_contract_addr
            )));
        }

        if !aux_contract_value.data.starts_with("0x") {
            return Err(GnosisError::ApiResultParse(format!(
                "aux contract data doesn't start with 0x: got {}",
                aux_contract_value.data,
            )));
        }

        let aux_data_bytes = hex::decode(&aux_contract_value.data[2..]).map_err(|err| {
            GnosisError::ApiResultParse(format!(
                "aux contract data {} cannot be hex-decoded: {}",
                aux_contract_value.data, err,
            ))
        })?;

        if !aux_data_bytes.starts_with(&audited_token.aux_burn_function_sig) {
            return Err(GnosisError::ApiResultParse(format!(
                "aux contract data {} does not start with the expected function signature",
                aux_contract_value.data,
            )));
        }

        // The tx out pub key is the last 32 bytes. Ensure we have enough bytes in the
        // data for that.
        let min_length = audited_token.aux_burn_function_sig.len() + 32;
        if aux_data_bytes.len() < min_length {
            return Err(GnosisError::ApiResultParse(format!(
                "aux contract data {} does not contain enough bytes. got {}, expected at least {}",
                aux_contract_value.data,
                aux_contract_value.data.len(),
                min_length,
            )));
        }

        let tx_out_pub_key = &aux_data_bytes[aux_data_bytes.len() - 32..];

        // Parsed everything we need.
        Ok(NewGnosisSafeWithdrawal {
            eth_tx_hash: multi_sig_tx.tx_hash.clone(),
            safe_address: multi_sig_tx.safe.to_string(),
            token_address: transfer_data.to.to_string(),
            amount: transfer_value as i64,
            mobilecoin_tx_out_public_key_hex: hex::encode(tx_out_pub_key),
        })
    }
}
