// Copyright (c) 2018-2022 The MobileCoin Foundation

use super::Conn;
use crate::{error::Error, gnosis::fetcher};
use diesel::prelude::*;

pub use super::models::GnosisSafeTx;

/// Trait for providing convenience functions for interacting with the
/// [GnosisSafeTx] model/table.
pub trait GnosisSafeTxModel {
    /// TODO
    fn insert(api_obj: &fetcher::GnosisSafeTransaction, conn: &Conn) -> Result<(), Error>;
}

impl GnosisSafeTxModel for GnosisSafeTx {
    fn insert(api_obj: &fetcher::GnosisSafeTransaction, conn: &Conn) -> Result<(), Error> {
        use super::schema::gnosis_safe_txs::dsl;

        let obj = GnosisSafeTx {
            eth_tx_hash: api_obj.tx_hash()?,
            raw_tx_json: api_obj.to_json_string(),
        };

        diesel::insert_into(dsl::gnosis_safe_txs)
            .values(obj)
            .execute(conn)?;

        Ok(())
    }
}
