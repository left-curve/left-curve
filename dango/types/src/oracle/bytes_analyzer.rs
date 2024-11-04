use {anyhow::bail, std::ops::Deref};

pub struct BytesAnalyzer {
    bytes: Vec<u8>,
    index: usize,
}

impl BytesAnalyzer {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, index: 0 }
    }

    /// Read the next byte.
    pub fn next_u8(&mut self) -> anyhow::Result<u8> {
        self.next_chunk::<1>().map(|array| array[0])
    }

    /// Read the next 2 bytes as a `u16` in big endian encoding.
    pub fn next_u16(&mut self) -> anyhow::Result<u16> {
        self.next_chunk::<2>().map(u16::from_be_bytes)
    }

    /// Read the next 4 bytes as a `u32` in big endian encoding.
    pub fn next_u32(&mut self) -> anyhow::Result<u32> {
        self.next_chunk::<4>().map(u32::from_be_bytes)
    }

    /// Read the next 8 bytes as a `u64` in big endian encoding.
    pub fn next_u64(&mut self) -> anyhow::Result<u64> {
        self.next_chunk::<8>().map(u64::from_be_bytes)
    }

    /// Read the next `S` bytes as an array.
    pub fn next_chunk<const S: usize>(&mut self) -> anyhow::Result<[u8; S]> {
        let Some(bytes) = self.bytes.get(self.index..self.index + S) else {
            bail!(
                "not enough bytes left! len: {}, index: {}, chunk size: {}",
                self.bytes.len(),
                self.index,
                S
            );
        };

        self.index += S;

        Ok(bytes.try_into()?)
    }

    /// Consume the analyzer, return the remaining, unread bytes as a vector.
    pub fn consume(mut self) -> Vec<u8> {
        self.bytes.split_off(self.index)
    }
}

impl Deref for BytesAnalyzer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes[self.index..]
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn byte_analyzer() {
        let raw = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut analyzer = BytesAnalyzer::new(raw);

        // next u8
        assert_eq!(analyzer.next_u8().unwrap(), 1);

        // next u16
        assert_eq!(analyzer.next_u16().unwrap(), u16::from_be_bytes([2, 3]));

        // next u32
        assert_eq!(
            analyzer.next_u32().unwrap(),
            u32::from_be_bytes([4, 5, 6, 7])
        );

        // deref
        assert_eq!(analyzer.deref(), &[8, 9, 10, 11, 12, 13, 14, 15]);

        // next chunk
        assert_eq!(analyzer.next_chunk::<4>().unwrap(), [8, 9, 10, 11]);

        // consume
        assert_eq!(analyzer.consume(), vec![12, 13, 14, 15]);
    }
}
