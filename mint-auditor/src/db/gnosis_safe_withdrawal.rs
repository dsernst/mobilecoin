// Copyright (c) 2018-2022 The MobileCoin Foundation

use super::Conn;
use crate::error::Error;
use diesel::prelude::*;

pub use super::models::{GnosisSafeWithdrawal, NewGnosisSafeWithdrawal};

/// Trait for providing convenience functions for interacting with the
/// [GnosisSafeWithdrawal] model/table.
pub trait GnosisSafeWithdrawalModel {
    /// TODO
    fn insert(obj: &NewGnosisSafeWithdrawal, conn: &Conn) -> Result<(), Error>;
}

impl GnosisSafeWithdrawalModel for GnosisSafeWithdrawal {
    fn insert(obj: &NewGnosisSafeWithdrawal, conn: &Conn) -> Result<(), Error> {
        use super::schema::gnosis_safe_withdrawals::dsl;

        diesel::insert_into(dsl::gnosis_safe_withdrawals)
            .values(obj)
            .execute(conn)?;

        Ok(())
    }
}
