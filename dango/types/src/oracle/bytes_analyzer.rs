use {anyhow::bail, paste::paste, std::ops::Deref};

pub struct BytesAnalyzer {
    bytes: Vec<u8>,
    index: usize,
}

macro_rules! impl_bytes {
    ($n:ty => $size:expr) => {
        paste! {
            #[doc = "Read the next {size} bytes as a `{n}` in big endian encoding."]
            pub fn [<next_ $n>](&mut self) -> anyhow::Result<$n> {
                if self.index + $size <= self.bytes.len() {
                    let bytes = &self.bytes[self.index..self.index + $size];
                    self.index += $size;
                    Ok(<$n>::from_be_bytes(bytes.try_into()?))
                } else {
                    bail!("Not enough bytes")
                }
            }
        }
    };
    ($($n:ty => $size:expr),+) => {
        $(
            impl_bytes! { $n => $size }
        )*
    };
}

impl BytesAnalyzer {
    impl_bytes! { u16 => 2, u32 => 4, u64 => 8 }

    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, index: 0 }
    }

    /// Read the next byte.
    pub fn next_u8(&mut self) -> u8 {
        self.index += 1;
        self.bytes[self.index - 1]
    }

    /// Read the next `S` bytes as an array.
    pub fn next_chunk<const S: usize>(&mut self) -> anyhow::Result<[u8; S]> {
        if self.index + S <= self.bytes.len() {
            let mut bytes: [u8; S] = [0; S];
            bytes.copy_from_slice(&self.bytes[self.index..self.index + S]);
            self.index += S;
            Ok(bytes)
        } else {
            bail!("Not enough bytes")
        }
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
        assert_eq!(analyzer.next_u8(), 1);

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
