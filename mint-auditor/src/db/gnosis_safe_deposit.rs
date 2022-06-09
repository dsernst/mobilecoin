// Copyright (c) 2018-2022 The MobileCoin Foundation

use super::{models::NewGnosisSafeDeposit, Conn};
use crate::{error::Error, gnosis::api_data_types::EthereumTransfer};
use diesel::prelude::*;

pub use super::models::GnosisSafeDeposit;

/// Trait for providing convenience functions for interacting with the
/// [GnosisSafeDeposit] model/table.
pub trait GnosisSafeDepositModel {
    /// Insert an Ethereum transfer as a deposit into the database.
    fn insert_eth_transfer(api_obj: &EthereumTransfer, conn: &Conn) -> Result<(), Error>;
}

impl GnosisSafeDepositModel for GnosisSafeDeposit {
    fn insert_eth_transfer(api_obj: &EthereumTransfer, conn: &Conn) -> Result<(), Error> {
        use super::schema::gnosis_safe_deposits::dsl;

        let obj = NewGnosisSafeDeposit {
            eth_tx_hash: api_obj.tx_hash.to_string(),
            safe_address: api_obj.to.to_string(),
            // Empty token address means ETH
            token_address: api_obj
                .token_address
                .clone()
                .unwrap_or_default()
                .to_string(),
            amount: u64::from(api_obj.value) as i64,
        };

        diesel::insert_into(dsl::gnosis_safe_deposits)
            .values(obj)
            .execute(conn)?;

        Ok(())
    }
}
