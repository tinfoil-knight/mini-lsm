#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::sync::Arc;
use std::{mem::size_of, path::Path};

use anyhow::Result;
use bytes::Bytes;

use super::{bloom::Bloom, BlockMeta, SsTable};
use crate::{
    block::BlockBuilder,
    key::{Key, KeySlice},
    lsm_storage::BlockCache,
    table::FileObject,
};

/// Builds an SSTable from key-value pairs.
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    keys: Vec<u32>,
    block_size: usize,
}

impl SsTableBuilder {
    /// Create a builder based on target block size.
    pub fn new(block_size: usize) -> Self {
        Self {
            builder: BlockBuilder::new(block_size),
            first_key: Vec::new(),
            last_key: Vec::new(),
            data: Vec::new(),
            meta: Vec::new(),
            keys: Vec::new(),
            block_size,
        }
    }

    /// Adds a key-value pair to SSTable.
    ///
    /// Note: You should split a new block when the current block is full.(`std::mem::replace` may
    /// be helpful here)
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        self.keys.push(farmhash::fingerprint32(key.raw_ref()));

        if self.builder.is_empty() {
            let _ = self.builder.add(key, value);
            self.first_key = key.raw_ref().to_vec();
        } else {
            let result = self.builder.add(key, value);
            if !result {
                let data_block =
                    std::mem::replace(&mut self.builder, BlockBuilder::new(self.block_size));
                self.meta.push(BlockMeta {
                    offset: self.data.len(),
                    first_key: Key::from_bytes(Bytes::copy_from_slice(&self.first_key)),
                    last_key: Key::from_bytes(Bytes::copy_from_slice(&self.last_key)),
                });
                self.data.append(&mut data_block.build().encode().to_vec());
                let _ = self.builder.add(key, value);
                self.first_key = key.raw_ref().to_vec();
            }
        }
        self.last_key = key.raw_ref().to_vec();
    }

    /// Get the estimated size of the SSTable.
    ///
    /// Since the data blocks contain much more data than meta blocks, just return the size of data
    /// blocks here.
    pub fn estimated_size(&self) -> usize {
        self.data.len()
    }

    /// Builds the SSTable and writes it to the given path. Use the `FileObject` structure to manipulate the disk objects.
    pub fn build(
        self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        // -----------------------------------------------------------------------------------------------------
        // |         Block Section         |                            Meta Section                           |
        // -----------------------------------------------------------------------------------------------------
        // | data block | ... | data block | metadata | meta block offset | bloom filter | bloom filter offset |
        // |                               |  varlen  |         u32       |    varlen    |        u32          |
        // -----------------------------------------------------------------------------------------------------

        let mut block_meta = self.meta;
        let mut data_blocks = self.data;

        if !self.builder.is_empty() {
            let offset = data_blocks.len();
            data_blocks.append(&mut self.builder.build().encode().to_vec());
            block_meta.push(BlockMeta {
                offset,
                first_key: Key::from_bytes(Bytes::copy_from_slice(&self.first_key)),
                last_key: Key::from_bytes(Bytes::copy_from_slice(&self.last_key)),
            });
        }
        let mut metadata_buf = Vec::new();
        BlockMeta::encode_block_meta(&block_meta, &mut metadata_buf);

        let block_meta_offset = data_blocks.len();

        let bits_per_key = Bloom::bloom_bits_per_key(self.keys.len(), 0.01);
        let bloom = Bloom::build_from_key_hashes(&self.keys, bits_per_key);
        let mut bloom_buf: Vec<u8> = Vec::new();
        bloom.encode(&mut bloom_buf);
        let bloom_offset = block_meta_offset + metadata_buf.len() + size_of::<u32>();

        let data = [
            data_blocks,
            metadata_buf,
            (block_meta_offset as u32).to_le_bytes().to_vec(),
            bloom_buf,
            (bloom_offset as u32).to_le_bytes().to_vec(),
        ]
        .concat();

        let (first_key, last_key) = (
            block_meta.first().unwrap().first_key.clone(),
            block_meta.last().unwrap().last_key.clone(),
        );

        Ok(SsTable {
            file: FileObject::create(path.as_ref(), data)?,
            block_meta,
            block_meta_offset,
            id,
            block_cache,
            first_key,
            last_key,
            bloom: Some(bloom),
            max_ts: 0,
        })
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }
}
