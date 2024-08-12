use crate::key::{KeySlice, KeyVec};

use super::Block;

/// Builds a block.
pub struct BlockBuilder {
    /// Offsets of each key-value entries.
    offsets: Vec<u16>,
    /// All serialized key-value pairs in the block.
    data: Vec<u8>,
    /// The expected block size.
    block_size: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockBuilder {
    /// Creates a new block builder.
    pub fn new(block_size: usize) -> Self {
        Self {
            offsets: Vec::new(),
            data: Vec::new(),
            block_size,
            first_key: KeyVec::new(),
        }
    }

    /// Adds a key-value pair to the block. Returns false when the block is full.
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        let (key_len, value_len) = (key.len(), value.len());
        let current_block_size = self.data.len();
        let increase = 2 + key_len + 2 + value_len + 2;
        let expected_block_size = current_block_size + increase + (self.offsets.len() + 1) * 2 + 2;

        let is_first = self.is_empty();
        if expected_block_size > self.block_size && !is_first {
            return false;
        }
        self.offsets.push(if is_first {
            0
        } else {
            current_block_size as u16
        });
        let pair = [
            &(key_len as u16).to_le_bytes(),
            key.into_inner(),
            &(value_len as u16).to_le_bytes(),
            value,
        ]
        .concat();
        self.data.extend(pair);
        if is_first {
            self.first_key = KeyVec::from_vec(key.into_inner().to_vec());
        }
        true
    }

    /// Check if there is no key-value pair in the block.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Finalize the block.
    pub fn build(self) -> Block {
        Block {
            data: self.data,
            offsets: self.offsets,
        }
    }
}
