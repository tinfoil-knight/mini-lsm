mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::Bytes;
pub use iterator::BlockIterator;

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
        // Each entry is a key-value pair.
        // -----------------------------------------------------------------------
        // |                           Entry #1                            | ... |
        // -----------------------------------------------------------------------
        // | key_len (2B) | key (keylen) | value_len (2B) | value (varlen) | ... |
        // -----------------------------------------------------------------------
        //
        // -------------------------------
        // |offset|offset|num_of_elements|
        // -------------------------------
        // |   0  |  12  |       2       |
        // -------------------------------
        // The footer of the block will be as above. Each of the number is stored as u16.

        let num_elements = self.offsets.len();
        let offsets: Vec<u8> = self.offsets.iter().flat_map(|x| x.to_le_bytes()).collect();
        Bytes::from(
            [
                self.data.as_slice(),
                &offsets,
                &(num_elements as u16).to_le_bytes(),
            ]
            .concat(),
        )
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        let (start, end) = (data.len() - 2, data.len());
        let num_elements = u16::from_le_bytes([data[start], data[end - 1]]);
        let offset_size = (num_elements * 2) as usize; // since offset is u16
        let (start, end) = (start - offset_size, start);
        let offsets: Vec<u16> = data[start..end]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let (start, end) = (0, start);

        Self {
            data: data[start..end].to_vec(),
            offsets,
        }
    }
}
