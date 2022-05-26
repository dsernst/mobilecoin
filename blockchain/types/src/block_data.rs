// Copyright (c) 2018-2022 The MobileCoin Foundation

use crate::{Block, BlockContents, BlockMetadata, BlockSignature};
use prost::Message;
use serde::{Deserialize, Serialize};

/// An object that holds all data included in and associated with a block.
#[derive(Clone, Deserialize, Eq, Message, PartialEq, Serialize)]
pub struct BlockData {
    #[prost(message, required, tag = 1)]
    /// The block header.
    pub block: Block,

    /// The block contents.
    #[prost(message, required, tag = 2)]
    pub contents: BlockContents,

    /// A signature over the [Block].
    #[prost(message, optional, tag = 3)]
    pub signature: Option<BlockSignature>,

    /// Block metadata.
    #[prost(message, optional, tag = 4)]
    pub metadata: Option<BlockMetadata>,
}

impl BlockData {
    /// Create new block data:
    ///
    /// Arguments:
    /// `block`: The block header
    /// `contents`: The block contents
    /// `signature`: An optional signature over the block
    /// `metadata`: Optional metadata for the block. This will become required
    ///     with a future BlockVersion.
    pub fn new(
        block: Block,
        contents: BlockContents,
        // Allows passing `signature`, `Some(signature)`, `None`.
        signature: impl Into<Option<BlockSignature>>,
        // Allows passing `metadata`, `Some(metadata)`, `None`.
        metadata: impl Into<Option<BlockMetadata>>,
    ) -> Self {
        Self {
            block,
            contents,
            signature: signature.into(),
            metadata: metadata.into(),
        }
    }

    /// Get the block.
    pub fn block(&self) -> &Block {
        &self.block
    }

    /// Get the contents.
    pub fn contents(&self) -> &BlockContents {
        &self.contents
    }

    /// Get the signature.
    pub fn signature(&self) -> &Option<BlockSignature> {
        &self.signature
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &Option<BlockMetadata> {
        &self.metadata
    }
}
