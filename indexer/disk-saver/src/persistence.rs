use {
    crate::error::Error,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{BorshDeExt, BorshSerExt},
    lzma_rs::{lzma_compress, lzma_decompress},
    std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
    },
    tempfile::NamedTempFile,
};

/// Leaving `should_compress` option for future use. Compressing takes more CPU and
/// might be annoying overtime. Maybe I'll store uncompressed first and compress
/// in its own task once the block has been indexed.
pub struct DiskPersistence {
    pub file_path: PathBuf,
    pub should_compress: bool,
}

impl DiskPersistence {
    pub fn new(mut file_path: PathBuf, should_compress: bool) -> Self {
        // let mut file_path = file_path.clone();
        file_path.set_extension("borsh");

        if should_compress {
            file_path.set_extension("xz");
        }

        Self {
            file_path,
            should_compress,
        }
    }

    /// Serialize and compress the data and save it to disk.
    pub fn save<T: BorshSerialize>(&self, data: &T) -> Result<(), Error> {
        let serialized = data.to_borsh_vec()?;

        let parent = Path::new(&self.file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let mut tmp_file = NamedTempFile::new_in(parent)?;

        if self.should_compress {
            let mut compressed = Vec::new();
            lzma_compress(&mut serialized.as_slice(), &mut compressed)?;
            tmp_file.write_all(&compressed)?;
        } else {
            tmp_file.write_all(&serialized)?;
        }

        tmp_file.flush()?;
        tmp_file.persist(&self.file_path)?;
        Ok(())
    }

    /// Load and decompress the data from disk and deserialize it.
    pub fn load<T: BorshDeserialize>(&self) -> Result<T, Error> {
        let disk_data = fs::read(&self.file_path)?;

        let data = if self.should_compress {
            let mut decompressed = Vec::new();
            lzma_decompress(&mut disk_data.as_slice(), &mut decompressed)?;

            decompressed.deserialize_borsh()?
        } else {
            disk_data.deserialize_borsh()?
        };

        Ok(data)
    }

    pub fn delete(&self) -> Result<(), Error> {
        std::fs::remove_file(&self.file_path)?;
        Ok(())
    }
}
