#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::sync::Arc;

use crate::key::{Key, KeySlice, KeyVec};

use super::Block;

/// Iterates on a block.
pub struct BlockIterator {
    /// The internal `Block`, wrapped by an `Arc`
    block: Arc<Block>,
    /// The current key, empty represents the iterator is invalid
    key: KeyVec,
    /// the current value range in the block.data, corresponds to the current key
    value_range: (usize, usize),
    /// Current index of the key-value pair, should be in range of [0, num_of_elements)
    idx: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
    }

    /// Creates a block iterator and seek to the first entry.
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        let mut itr = Self::new(block);
        itr.seek_to_first();
        itr
    }

    /// Creates a block iterator and seek to the first key that >= `key`.
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        let mut itr = Self::new(block);
        itr.seek_to_key(key);
        itr
    }

    /// Returns the key of the current entry.
    pub fn key(&self) -> KeySlice {
        self.key.as_key_slice()
    }

    /// Returns the value of the current entry.
    pub fn value(&self) -> &[u8] {
        let (start, end) = self.value_range;
        &self.block.data[start..end]
    }

    /// Returns true if the iterator is valid.
    /// Note: You may want to make use of `key`
    pub fn is_valid(&self) -> bool {
        self.key.is_empty()
    }

    /// Seeks to the first key in the block.
    pub fn seek_to_first(&mut self) {
        self.first_key.clear();
        self.seek_to_idx(0);
    }

    /// Move to the next key in the block.
    pub fn next(&mut self) {
        if self.idx + 1 >= self.block.offsets.len() {
            self.key.clear();
            return;
        }
        self.seek_to_idx(self.idx + 1);
    }

    /// Seek to the first key that >= `key`.
    /// Note: You should assume the key-value pairs in the block are sorted when being added by
    /// callers.
    pub fn seek_to_key(&mut self, key: KeySlice) {
        for (i, offset) in self.block.offsets.iter().enumerate() {
            let start = *offset as usize;
            let data = &self.block.data;
            let key_len = u16::from_le_bytes([data[start], data[start + 1]]);
            let end = start + 2 + key_len as usize;
            let key_from_data = Key::from_slice(&data[start + 2..end]);
            if key_from_data >= key {
                self.seek_to_idx(i);
                return;
            }
        }

        self.key.clear()
    }

    fn seek_to_idx(&mut self, idx: usize) {
        let block = &self.block;
        let start = block.offsets[idx] as usize;
        let data = &block.data;

        let key_len = u16::from_le_bytes([data[start], data[start + 1]]);
        let end = start + 2 + key_len as usize;
        let key = &data[start + 2..end];
        self.key.set_from_slice(KeySlice::from_slice(key));

        let value_len = u16::from_le_bytes([data[end], data[end + 1]]);
        let start = end + 2;
        self.value_range = (start, start + value_len as usize);

        if self.first_key.is_empty() {
            self.first_key = KeyVec::from_vec(key.to_vec());
            self.idx = 0;
        } else {
            self.idx += 1
        }
    }
}
