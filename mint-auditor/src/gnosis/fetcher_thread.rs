// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Background thread for periodically fetching data from the Gnosis API.

use super::{
    api_data_types::{EthereumTransaction, MultiSigTransaction, Transaction},
    fetcher::{GnosisSafeFetcher, GnosisSafeTransaction},
    SafeAddr,
};
use crate::{
    db::{
        transaction, Conn, GnosisSafeDeposit, GnosisSafeDepositModel, GnosisSafeTx,
        GnosisSafeTxModel, GnosisSafeWithdrawal, GnosisSafeWithdrawalModel, MintAuditorDb,
        NewGnosisSafeWithdrawal,
    },
    error::Error,
};
use mc_common::logger::{log, Logger};
use mc_ledger_db::LedgerDB;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};
use url::Url;

// TODO make these configurable, support multiple ones
const TOKEN1_CONTRACT_ADDRESS: &str = "0xD92E713d051C37EbB2561803a3b5FBAbc4962431"; // TUSDT
const MOBILECOIN_AUX_CONTRACT_ADDRESS: &str = "0x76BD419fBa96583d968b422D4f3CB2A70bf4CF40"; // The test contract that holds the public key

pub struct FetcherThread {
    stop_requested: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
    logger: Logger,
}

impl FetcherThread {
    pub fn start(
        safe_addr: SafeAddr,
        mint_auditor_db: MintAuditorDb,
        ledger_db: LedgerDB,
        poll_interval: Duration,
        gnosis_api_url: Url,
        logger: Logger,
    ) -> Result<Self, Error> {
        let fetcher = GnosisSafeFetcher::new(gnosis_api_url, logger.clone())?;
        let stop_requested = Arc::new(AtomicBool::new(false));

        let thread_stop_requested = stop_requested.clone();
        let thread_logger = logger.clone();

        let join_handle = Some(spawn(move || {
            thread_entry_point(
                thread_stop_requested,
                safe_addr,
                mint_auditor_db,
                ledger_db,
                poll_interval,
                fetcher,
                thread_logger,
            )
        }));

        Ok(Self {
            stop_requested,
            join_handle,
            logger,
        })
    }
    pub fn stop(&mut self) {
        log::info!(self.logger, "Stopping fetcher thread...");
        self.stop_requested.store(true, Ordering::Relaxed);
        if let Some(join_nandle) = self.join_handle.take() {
            join_nandle
                .join()
                .expect("failed joining gnosis fetcher thread");
        }
    }
}

impl Drop for FetcherThread {
    fn drop(&mut self) {
        self.stop();
    }
}

fn thread_entry_point(
    stop_requested: Arc<AtomicBool>,
    safe_addr: SafeAddr,
    mint_auditor_db: MintAuditorDb,
    _ledger_db: LedgerDB,
    poll_interval: Duration,
    fetcher: GnosisSafeFetcher,
    logger: Logger,
) {
    log::info!(logger, "GnosisFetcher thread started");
    let worker = FetcherThreadWorker {
        safe_addr: safe_addr.clone(),
        mint_auditor_db,
        logger: logger.clone(),
    };

    loop {
        if stop_requested.load(Ordering::Relaxed) {
            log::info!(logger, "GnosisFetcher thread stop trigger received");
            break;
        }

        // TODO handle pagination (offset, limit)
        match fetcher.get_transaction_data(&safe_addr) {
            Ok(transactions) => {
                worker.process_transactions(transactions);
            }
            Err(err) => {
                log::error!(logger, "Failed to fetch Gnosis transactions: {}", err);
            }
        }

        sleep(poll_interval);
    }
}

struct FetcherThreadWorker {
    safe_addr: SafeAddr,
    mint_auditor_db: MintAuditorDb,
    logger: Logger,
}

impl FetcherThreadWorker {
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
                    _ => {
                        log::info!(self.logger, "TODO {:?}", tx);
                    }
                };

                Ok(())
            })
            .expect("failed processing transaction");
        }
    }

    pub fn process_eth_transaction(
        &self,
        conn: &Conn,
        tx: &EthereumTransaction,
    ) -> Result<(), Error> {
        log::trace!(self.logger, "Processing Ethereum transaction: {:?}", tx);

        for transfer in &tx.transfers {
            // See if this is a deposit to the safe.
            if transfer.to == self.safe_addr {
                log::info!(
                    self.logger,
                    "Processing gnosis safe deposit: {:?}",
                    transfer
                );
                GnosisSafeDeposit::insert_eth_transfer(transfer, &conn)?;

                // TODO this is the TUSDT contract address
                // if token_address == Some("0xd92e713d051c37ebb2561803a3b5fbabc4962431") {
                //     log::info!(self.logger, "TODO: deposit to safe");
                // } else {
                //     log::error!(self.logger, "")
                // }
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

    pub fn process_multi_sig_transaction(
        &self,
        conn: &Conn,
        multi_sig_tx: &MultiSigTransaction,
    ) -> Result<(), Error> {
        if let Some(withdrawal) = self.parse_withdrawal_with_pub_key_multi_sig_tx(multi_sig_tx) {
            log::info!(
                self.logger,
                "Processing withdrawal from multi-sig tx: {:?}",
                withdrawal
            );

            GnosisSafeWithdrawal::insert(&withdrawal, conn)?;
        }

        Ok(())
    }

    pub fn parse_withdrawal_with_pub_key_multi_sig_tx(
        &self,
        multi_sig_tx: &MultiSigTransaction,
    ) -> Option<NewGnosisSafeWithdrawal> {
        // See if this is a multi-sig withdrawal that uses the auxiliary contract for
        // recording the tx out public key.
        let data = multi_sig_tx.data_decoded.as_ref()?;

        if data.method != "multiSend" {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with method {}",
                multi_sig_tx.tx_hash,
                data.method
            );
            return None;
        }

        if data.parameters.len() != 1 {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with {} parameters",
                multi_sig_tx.tx_hash,
                data.parameters.len()
            );
            return None;
        }

        let parameter = &data.parameters[0];
        let value_decoded = if let Some(val) = parameter.value_decoded.as_ref() {
            val
        } else {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with no value",
                multi_sig_tx.tx_hash
            );
            return None;
        };

        if value_decoded.len() != 2 {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with {} value parameters",
                multi_sig_tx.tx_hash,
                value_decoded.len()
            );
            return None;
        }

        let transfer_data = &value_decoded[0];
        if transfer_data.to != TOKEN1_CONTRACT_ADDRESS {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with transfer to {}",
                multi_sig_tx.tx_hash,
                transfer_data.to
            );
            return None;
        }
        let transfer_data_decoded = if let Some(data) = transfer_data.data_decoded.as_ref() {
            data
        } else {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with no transfer data",
                multi_sig_tx.tx_hash
            );
            return None;
        };
        if transfer_data_decoded.method != "transfer" {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with transfer method {}",
                multi_sig_tx.tx_hash,
                transfer_data_decoded.method
            );
            return None;
        }
        let value_str = if let Some(val) =
            transfer_data_decoded.parameters.iter().find_map(|param| {
                if param.name == "value" {
                    Some(&param.value)
                } else {
                    None
                }
            }) {
            val
        } else {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with no value parameter",
                multi_sig_tx.tx_hash
            );
            return None;
        };
        let transfer_value = if let Ok(val) = value_str.parse::<u64>() {
            val
        } else {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with invalid value {}",
                multi_sig_tx.tx_hash,
                value_str
            );
            return None;
        };

        let aux_contract_value = &value_decoded[1];
        if aux_contract_value.to != MOBILECOIN_AUX_CONTRACT_ADDRESS {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with aux contract to {}",
                multi_sig_tx.tx_hash,
                aux_contract_value.to
            );
            return None;
        }

        if !aux_contract_value.data.starts_with("0x") {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with invalid aux contract value {}",
                multi_sig_tx.tx_hash,
                aux_contract_value.data,
            );
            return None;
        }
        let aux_data_bytes = if let Ok(bytes) = hex::decode(&aux_contract_value.data[2..]) {
            bytes
        } else {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with invalid aux data {}",
                multi_sig_tx.tx_hash,
                aux_contract_value.data,
            );
            return None;
        };

        if aux_data_bytes.len() != 100 || !aux_data_bytes.starts_with(b"\xc7\x6f\x06\x35") {
            log::info!(
                self.logger,
                "Skipping multi-sig tx {} with invalid aux data {}",
                multi_sig_tx.tx_hash,
                aux_contract_value.data,
            );
            return None;
        }

        // The tx out pub key is the last 32 bytes.
        let tx_out_pub_key = &aux_data_bytes[aux_data_bytes.len() - 32..];

        Some(NewGnosisSafeWithdrawal {
            eth_tx_hash: multi_sig_tx.tx_hash.clone(),
            safe_address: multi_sig_tx.safe.clone(),
            token_address: transfer_data.to.clone(),
            amount: transfer_value as i64,
            mobilecoin_tx_out_public_key_hex: hex::encode(tx_out_pub_key),
        })
    }
}
