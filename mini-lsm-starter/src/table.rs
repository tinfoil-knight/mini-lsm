#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

pub(crate) mod bloom;
mod builder;
mod iterator;

use std::cmp::min;
use std::fs::File;
use std::mem::size_of;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
pub use builder::SsTableBuilder;
use bytes::{Buf, Bytes};
pub use iterator::SsTableIterator;

use crate::block::Block;
use crate::key::{KeyBytes, KeySlice};
use crate::lsm_storage::BlockCache;

use self::bloom::Bloom;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMeta {
    /// Offset of this data block.
    pub offset: usize,
    /// The first key of the data block.
    pub first_key: KeyBytes,
    /// The last key of the data block.
    pub last_key: KeyBytes,
}

impl BlockMeta {
    /// Encode block meta to a buffer.
    /// You may add extra fields to the buffer,
    /// in order to help keep track of `first_key` when decoding from the same buffer in the future.
    pub fn encode_block_meta(block_meta: &[BlockMeta], buf: &mut Vec<u8>) {
        // BlockMeta should include the first/last keys in each block and the offsets of each block.
        // Format: |offset|f_key_len|first_key|l_key_len|last_key|
        let usize_to_bytes = |x: usize| -> [u8; 8] { (x as u64).to_le_bytes() };
        buf.append(
            &mut block_meta
                .iter()
                .flat_map(|bm| {
                    [
                        usize_to_bytes(bm.offset).as_slice(),
                        usize_to_bytes(bm.first_key.len()).as_slice(),
                        bm.first_key.raw_ref(),
                        usize_to_bytes(bm.last_key.len()).as_slice(),
                        bm.last_key.raw_ref(),
                    ]
                    .concat()
                })
                .collect(),
        );
    }

    /// Decode block meta from a buffer.
    pub fn decode_block_meta(buf: impl Buf) -> Vec<BlockMeta> {
        let mut buf = buf.chunk();
        let mut v = Vec::new();
        let bytes_to_usize = |x: &[u8]| usize::from_le_bytes(x[..8].try_into().unwrap());
        while buf.has_remaining() {
            let offset = bytes_to_usize(buf.chunk());
            buf.advance(8);

            let key_len = bytes_to_usize(buf.chunk());
            buf.advance(8);
            let first_key = KeyBytes::from_bytes(Bytes::copy_from_slice(&buf.chunk()[..key_len]));
            buf.advance(key_len);

            let key_len = bytes_to_usize(buf.chunk());
            buf.advance(8);
            let last_key = KeyBytes::from_bytes(Bytes::copy_from_slice(&buf.chunk()[..key_len]));
            buf.advance(key_len);

            v.push(Self {
                offset,
                first_key,
                last_key,
            })
        }
        v
    }
}

/// A file object.
pub struct FileObject(Option<File>, u64);

impl FileObject {
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;
        let mut data = vec![0; len as usize];
        self.0
            .as_ref()
            .unwrap()
            .read_exact_at(&mut data[..], offset)?;
        Ok(data)
    }

    pub fn size(&self) -> u64 {
        self.1
    }

    /// Create a new file object (day 2) and write the file to the disk (day 4).
    pub fn create(path: &Path, data: Vec<u8>) -> Result<Self> {
        std::fs::write(path, &data)?;
        File::open(path)?.sync_all()?;
        Ok(FileObject(
            Some(File::options().read(true).write(false).open(path)?),
            data.len() as u64,
        ))
    }

    pub fn open(path: &Path) -> Result<Self> {
        let file = File::options().read(true).write(false).open(path)?;
        let size = file.metadata()?.len();
        Ok(FileObject(Some(file), size))
    }
}

/// An SSTable.
pub struct SsTable {
    /// The actual storage unit of SsTable, the format is as above.
    pub(crate) file: FileObject,
    /// The meta blocks that hold info for data blocks.
    pub(crate) block_meta: Vec<BlockMeta>,
    /// The offset that indicates the start point of meta blocks in `file`.
    pub(crate) block_meta_offset: usize,
    id: usize,
    block_cache: Option<Arc<BlockCache>>,
    first_key: KeyBytes,
    last_key: KeyBytes,
    pub(crate) bloom: Option<Bloom>,
    /// The maximum timestamp stored in this SST, implemented in week 3.
    max_ts: u64,
}

impl SsTable {
    #[cfg(test)]
    pub(crate) fn open_for_test(file: FileObject) -> Result<Self> {
        Self::open(0, None, file)
    }

    /// Open SSTable from a file.
    pub fn open(id: usize, block_cache: Option<Arc<BlockCache>>, file: FileObject) -> Result<Self> {
        // -----------------------------------------------------------------------------------------------------
        // |         Block Section         |                            Meta Section                           |
        // -----------------------------------------------------------------------------------------------------
        // | data block | ... | data block | metadata | meta block offset | bloom filter | bloom filter offset |
        // |                               |  varlen  |         u32       |    varlen    |        u32          |
        // -----------------------------------------------------------------------------------------------------

        let offset = file.size() - size_of::<u32>() as u64;
        let bloom_offset = file
            .read(offset, size_of::<u32>() as u64)?
            .try_into()
            .map_or(0, u32::from_le_bytes);
        let bloom_len = offset - bloom_offset as u64;

        let bloom = Bloom::decode(file.read(bloom_offset as u64, bloom_len)?.as_slice());

        let block_meta_offset = file
            .read(
                bloom_offset as u64 - size_of::<u32>() as u64,
                size_of::<u32>() as u64,
            )?
            .try_into()
            .map_or(0, u32::from_le_bytes);
        let block_meta_len =
            bloom_offset as u64 - size_of::<u32>() as u64 - block_meta_offset as u64;

        let block_meta = BlockMeta::decode_block_meta(
            file.read(block_meta_offset as u64, block_meta_len)?
                .as_slice(),
        );

        let (first_key, last_key) = (
            block_meta
                .first()
                .map_or(KeyBytes::default(), |v| v.first_key.clone()),
            block_meta
                .last()
                .map_or(KeyBytes::default(), |v| v.last_key.clone()),
        );
        Ok(Self {
            file,
            block_meta,
            block_meta_offset: block_meta_offset as usize,
            id,
            block_cache,
            first_key,
            last_key,
            bloom: bloom.ok(),
            max_ts: 0,
        })
    }

    /// Create a mock SST with only first key + last key metadata
    pub fn create_meta_only(
        id: usize,
        file_size: u64,
        first_key: KeyBytes,
        last_key: KeyBytes,
    ) -> Self {
        Self {
            file: FileObject(None, file_size),
            block_meta: vec![],
            block_meta_offset: 0,
            id,
            block_cache: None,
            first_key,
            last_key,
            bloom: None,
            max_ts: 0,
        }
    }

    /// Read a block from the disk.
    pub fn read_block(&self, block_idx: usize) -> Result<Arc<Block>> {
        let metadata = &self.block_meta[block_idx];
        let len = self
            .block_meta
            .get(block_idx + 1)
            .map_or(self.block_meta_offset, |v| v.offset)
            - metadata.offset;
        let data = self.file.read(metadata.offset as u64, len as u64)?;
        let block = Arc::new(Block::decode(&data));
        Ok(block)
    }

    /// Read a block from disk, with block cache. (Day 4)
    pub fn read_block_cached(&self, block_idx: usize) -> Result<Arc<Block>> {
        // SEEN
        match &self.block_cache {
            Some(cache) => {
                let blk = cache
                    .try_get_with((self.id, block_idx), || self.read_block(block_idx))
                    .map_err(|e| anyhow!("{}", e))?;
                Ok(blk)
            }
            None => self.read_block(block_idx),
        }
    }

    /// Find the block that may contain `key`.
    /// Note: You may want to make use of the `first_key` stored in `BlockMeta`.
    /// You may also assume the key-value pairs stored in each consecutive block are sorted.
    pub fn find_block_idx(&self, key: KeySlice) -> usize {
        match self
            .block_meta
            .binary_search_by_key(&key, |metatdata| metatdata.last_key.as_key_slice())
        {
            Ok(idx) => idx,
            Err(idx) => min(idx, self.num_of_blocks() - 1),
        }
    }

    /// Get number of data blocks.
    pub fn num_of_blocks(&self) -> usize {
        self.block_meta.len()
    }

    pub fn first_key(&self) -> &KeyBytes {
        &self.first_key
    }

    pub fn last_key(&self) -> &KeyBytes {
        &self.last_key
    }

    pub fn table_size(&self) -> u64 {
        self.file.1
    }

    pub fn sst_id(&self) -> usize {
        self.id
    }

    pub fn max_ts(&self) -> u64 {
        self.max_ts
    }
}
