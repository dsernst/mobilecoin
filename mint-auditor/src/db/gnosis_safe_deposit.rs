// Copyright (c) 2018-2022 The MobileCoin Foundation

use super::{models::NewGnosisSafeDeposit, Conn};
use crate::{error::Error, gnosis::api_data_types};
use diesel::prelude::*;

pub use super::models::GnosisSafeDeposit;

/// Trait for providing convenience functions for interacting with the
/// [GnosisSafeDeposit] model/table.
pub trait GnosisSafeDepositModel {
    /// TODO
    fn insert_eth_transfer(
        api_obj: &api_data_types::EthereumTransfer,
        conn: &Conn,
    ) -> Result<(), Error>;
}

impl GnosisSafeDepositModel for GnosisSafeDeposit {
    fn insert_eth_transfer(
        api_obj: &api_data_types::EthereumTransfer,
        conn: &Conn,
    ) -> Result<(), Error> {
        use super::schema::gnosis_safe_deposits::dsl;

        let obj = NewGnosisSafeDeposit {
            eth_tx_hash: api_obj.tx_hash.clone(),
            safe_address: api_obj.to.to_string(),
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
