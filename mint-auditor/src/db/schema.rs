table! {
    block_audit_data (id) {
        id -> Nullable<Integer>,
        block_index -> BigInt,
    }
}

table! {
    block_balance (id) {
        id -> Nullable<Integer>,
        block_index -> BigInt,
        token_id -> BigInt,
        balance -> BigInt,
    }
}

table! {
    counters (id) {
        id -> Integer,
        num_blocks_synced -> BigInt,
        num_burns_exceeding_balance -> BigInt,
        num_mint_txs_without_matching_mint_config -> BigInt,
    }
}

table! {
    gnosis_safe_deposits (id) {
        id -> Nullable<Integer>,
        eth_tx_hash -> Text,
        safe_address -> Text,
        token_address -> Text,
        amount -> BigInt,
    }
}

table! {
    gnosis_safe_txs (eth_tx_hash) {
        eth_tx_hash -> Text,
        raw_tx_json -> Text,
    }
}

table! {
    gnosis_safe_withdrawals (id) {
        id -> Nullable<Integer>,
        eth_tx_hash -> Text,
        safe_address -> Text,
        token_address -> Text,
        amount -> BigInt,
        mobilecoin_tx_out_public_key_hex -> Text,
    }
}

table! {
    mobilecoin_mint_txs (id) {
        id -> Nullable<Integer>,
        block_index -> BigInt,
        token_id -> BigInt,
        amount -> Integer,
        recipient_b58_address -> Text,
        nonce_hex -> Text,
        tombstone_block -> BigInt,
    }
}

joinable!(gnosis_safe_deposits -> gnosis_safe_txs (eth_tx_hash));
joinable!(gnosis_safe_withdrawals -> gnosis_safe_txs (eth_tx_hash));

allow_tables_to_appear_in_same_query!(
    block_audit_data,
    block_balance,
    counters,
    gnosis_safe_deposits,
    gnosis_safe_txs,
    gnosis_safe_withdrawals,
    mobilecoin_mint_txs,
);
