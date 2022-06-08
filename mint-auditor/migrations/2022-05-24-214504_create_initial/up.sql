-- Audit data per block
CREATE TABLE block_audit_data (
    -- Diesel requires having a primary key and sqlite doesn't allow 64 bit primay keys, so even though
    -- we would've wanted to use the block_index for that we can't.
    -- Must be nullable for auto-increment: https://www.sqlite.org/autoinc.html
    id INTEGER PRIMARY KEY,
    block_index UNSIGNED BIGINT NOT NULL
    -- Future revision would add gnosis safe data here
);
CREATE UNIQUE INDEX idx__block_audit_data__block_index ON block_audit_data(block_index);

-- Balance per token, for each block we have audited.
CREATE TABLE block_balance (
    -- Diesel requires having a primary key and sqlite doesn't allow 64 bit primay keys, so even though
    -- we would've wanted to use the block_index for that we can't.
    -- Must be nullable for auto-increment: https://www.sqlite.org/autoinc.html
    id INTEGER PRIMARY KEY,
    block_index UNSIGNED BIGINT NOT NULL,
    token_id UNSIGNED BIGINT NOT NULL,
    balance UNSIGNED BIGINT NOT NULL,
    FOREIGN KEY (block_index) REFERENCES block_audit_data(block_index)
);
CREATE UNIQUE INDEX idx__block_balance__block_index__token_id ON block_balance(block_index, token_id);

-- Mints on the mobilecoin blockchain
CREATE TABLE mobilecoin_mint_txs (
    -- Diesel requires having a primary key and sqlite doesn't allow 64 bit primay keys, so even though
    -- we would've wanted to use the block_index for that we can't.
    -- Must be nullable for auto-increment: https://www.sqlite.org/autoinc.html
    id INTEGER PRIMARY KEY,
    block_index UNSIGNED BIGINT NOT NULL,
    token_id UNSIGNED BIGINT NOT NULL,
    amount UNSIGNED BITINT NOT NULL,
    recipient_b58_address VARCHAR NOT NULL,
    nonce_hex VARCHAR NOT NULL UNIQUE,
    tombstone_block UNSIGNED BIGINT NOT NULL,
    FOREIGN KEY (block_index) REFERENCES block_audit_data(block_index)
);

-- Processed gnosis safe transactions
CREATE TABLE gnosis_safe_txs (
    eth_tx_hash VARCHAR NOT NULL UNIQUE PRIMARY KEY,
    raw_tx_json TEXT NOT NULL
);

-- Deposits to the gnosis safe.
CREATE TABLE gnosis_safe_deposits (
    -- Diesel requires having a primary key and sqlite doesn't allow 64 bit primay keys, so even though
    -- we would've wanted to use the block_index for that we can't.
    -- Must be nullable for auto-increment: https://www.sqlite.org/autoinc.html
    id INTEGER PRIMARY KEY,
    eth_tx_hash VARCHAR NOT NULL,
    safe_address VARCHAR NOT NULL,
    token_address VARCHAR NOT NULL,
    amount UNSIGNED BIGINT NOT NULL,
    FOREIGN KEY (eth_tx_hash) REFERENCES gnosis_safe_txs(eth_tx_hash)
);

-- Withdrawals from the gnosis safe.
CREATE TABLE gnosis_safe_withdrawals (
    -- Diesel requires having a primary key and sqlite doesn't allow 64 bit primay keys, so even though
    -- we would've wanted to use the block_index for that we can't.
    -- Must be nullable for auto-increment: https://www.sqlite.org/autoinc.html
    id INTEGER PRIMARY KEY,
    eth_tx_hash VARCHAR NOT NULL,
    safe_address VARCHAR NOT NULL,
    token_address VARCHAR NOT NULL,
    amount UNSIGNED BIGINT NOT NULL,
    mobilecoin_tx_out_public_key_hex VARCHAR NOT NULL,
    FOREIGN KEY (eth_tx_hash) REFERENCES gnosis_safe_txs(eth_tx_hash)
);
CREATE INDEX idx__gnosis_safe_withdrawals__mobilecoin_tx_out_public_key_hex ON gnosis_safe_withdrawals(mobilecoin_tx_out_public_key_hex);

-- Counters - this table is expected to only ever have a single row.
CREATE TABLE counters (
    -- Diesel only supports tables with primary keys, so we need one.
    -- Not nullable because we only have a single row in this table and the code that inserts to it hard-codes the id to 0.
    id INTEGER NOT NULL PRIMARY KEY,

    -- Number of blocks we've synced so far.
    num_blocks_synced UNSIGNED BIGINT NOT NULL,

    -- Number of times we've encountered a burn that exceeds the calculated balance.
    num_burns_exceeding_balance UNSIGNED BIGINT NOT NULL,

    -- Number of `MintTx`s that did not match an active mint config.
    num_mint_txs_without_matching_mint_config UNSIGNED BIGINT NOT NULL
);

