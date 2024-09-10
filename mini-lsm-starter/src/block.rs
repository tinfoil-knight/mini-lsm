mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::Bytes;
pub use iterator::BlockIterator;

pub(crate) const SIZEOF_U16: usize = std::mem::size_of::<u16>();

/// A block is the smallest unit of read and caching in LSM tree. It is a collection of sorted key-value pairs.
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    /// Encode the internal data to the data layout illustrated in the tutorial
    /// Note: You may want to recheck if any of the expected field is missing from your output
    pub fn encode(&self) -> Bytes {
        // Block encoding format
        // ----------------------------------------------------------------------------------------------------
        // |             Data Section             |              Offset Section             |      Extra      |
        // ----------------------------------------------------------------------------------------------------
        // | Entry #1 | Entry #2 | ... | Entry #N | Offset #1 | Offset #2 | ... | Offset #N | num_of_elements |
        // ----------------------------------------------------------------------------------------------------
        //
        // Offsets & num_of_elements are stored as u16.
        //
        // Each entry is a key-value pair.
        // -----------------------------------------------------------------------
        // |                           Entry #1                            | ... |
        // -----------------------------------------------------------------------
        // | key_len (2B) | key (keylen) | value_len (2B) | value (varlen) | ... |
        // -----------------------------------------------------------------------

        let mut buf = self.data.clone();
        let num_elements = self.offsets.len();

        for offset in &self.offsets {
            buf.extend_from_slice(&offset.to_le_bytes())
        }
        buf.extend_from_slice(&(num_elements as u16).to_le_bytes());
        buf.into()
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        // As key-value entries are stored in raw format and offsets are stored in a separate vector,
        // this reduces unnecessary memory allocations and processing overhead when decoding data ——
        // what you need to do is to simply copy the raw block data to the data vector and decode the entry offsets every 2 bytes,
        // instead of creating something like Vec<(Vec<u8>, Vec<u8>)> to store all the key-value pairs in one block in memory.

        let (start, end) = (data.len() - SIZEOF_U16, data.len());
        let num_elements = u16::from_le_bytes([data[start], data[end - 1]]);
        let offset_size = num_elements as usize * SIZEOF_U16;
        let (start, end) = (start - offset_size, start);
        let offsets: Vec<u16> = data[start..end]
            .chunks_exact(SIZEOF_U16)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let (start, end) = (0, start);

        Self {
            data: data[start..end].to_vec(),
            offsets,
        }
    }
}
