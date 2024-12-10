use {
    crate::error::Error,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{BorshDeExt, BorshSerExt},
    lzma_rs::{lzma_compress, lzma_decompress},
    std::{fs, io::Write, path::Path},
    tempfile::NamedTempFile,
};
pub struct DiskPersistence {
    pub file_path: String,
}

impl DiskPersistence {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }

    /// Serialize and compress the data and save it to disk.
    pub fn save<T: BorshSerialize>(&self, data: &T) -> Result<(), Error> {
        let serialized = data.to_borsh_vec()?;

        let mut compressed = Vec::new();
        lzma_compress(&mut serialized.as_slice(), &mut compressed)?;

        let parent = Path::new(&self.file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let mut tmp_file = NamedTempFile::new_in(parent)?;
        tmp_file.write_all(&compressed)?;
        tmp_file.flush()?;
        tmp_file.persist(&self.file_path)?;
        Ok(())
    }

    /// Load and decompress the data from disk and deserialize it.
    pub fn load<T: BorshDeserialize>(&self) -> Result<T, Error> {
        let compressed = fs::read(&self.file_path)?;
        let mut decompressed = Vec::new();
        lzma_decompress(&mut compressed.as_slice(), &mut decompressed)?;

        let data = decompressed.deserialize_borsh()?; //  T::try_from_slice(&decompressed)?;
        Ok(data)
    }
}
