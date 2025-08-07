use {
    crate::error::Error,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{BorshDeExt, BorshSerExt},
    lzma_rs::{lzma_compress, lzma_decompress},
    std::{
        fs::{self, OpenOptions},
        io::{BufReader, BufWriter, Write},
        path::{Path, PathBuf},
    },
    tempfile::NamedTempFile,
};

/// Leaving `should_compress` option for future use. Compressing takes more CPU and
/// might be annoying overtime. Maybe I'll store uncompressed first and compress
/// in its own task once the block has been indexed.
pub struct DiskPersistence {
    pub file_path: PathBuf,
    pub compressed: bool,
}

impl DiskPersistence {
    /// Will automatically set the file extension based on the serialization and compression
    pub fn new(mut file_path: PathBuf, mut should_compress: bool) -> Self {
        // For if I'll support other serialization mechanism so we can figure
        // out the serialization format based on the file extension.
        file_path.set_extension("borsh");

        let mut compressed_file_path = file_path.clone();
        compressed_file_path.set_extension("borsh.xz");

        match (
            should_compress,
            file_path.exists(),
            compressed_file_path.exists(),
        ) {
            (_, true, true) => {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    file_path = %file_path.display(),
                    compressed_file_path = %compressed_file_path.display(),
                    "Both compressed and uncompressed file exists."
                );

                should_compress = true;
            },
            // Compressed file exists, we'll use it.
            (_, _, true) => {
                should_compress = true;
                file_path = compressed_file_path;
            },
            // Uncompressed file exists, we'll use it.
            (_, true, _) => {
                should_compress = false;
            },
            // No file exists, we'll create an compressed file.
            (true, ..) => {
                file_path = compressed_file_path;
            },
            // No file exists, we'll create an uncompressed file.
            (false, ..) => {},
        }

        Self {
            file_path,
            compressed: should_compress,
        }
    }

    fn detect_file_existence(&mut self) {
        let mut file_path = self.file_path.clone();

        while file_path.extension().is_some() {
            file_path.set_extension("");
        }

        file_path.set_extension("borsh");

        let mut compressed_file_path = file_path.clone();
        compressed_file_path.set_extension("borsh.xz");

        if compressed_file_path.exists() {
            self.file_path = compressed_file_path;
            self.compressed = true;
        } else {
            self.file_path = file_path;
            self.compressed = false;
        }
    }

    /// Serialize and compress the data and save it to disk.
    pub fn save<T: BorshSerialize>(&self, data: &T) -> Result<(), Error> {
        // Block are final, we don't want to overwrite them.
        // NOTE: we might need a `force: true` option in the future but I don't see
        // why we would want to overwrite a block.
        if self.file_path.exists() {
            #[cfg(feature = "tracing")]
            tracing::warn!(file_path = %self.file_path.display(), "File already exists, saving anyway");
        }

        let serialized = data.to_borsh_vec()?;

        let parent = Path::new(&self.file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));

        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }

        let mut tmp_file = NamedTempFile::new_in(parent)?;

        if self.compressed {
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

    pub fn compress(&mut self) -> Result<bool, Error> {
        if self.compressed {
            return Ok(false);
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(?self.file_path, "Compressing file");

        // Define the path for the compressed file
        let mut compressed_path = self.file_path.clone();
        compressed_path.set_extension("borsh.xz");

        // This shouldn't happen since if compressed file already exists,
        // we should have `self.compressed` to true.
        if compressed_path.exists() {
            #[cfg(feature = "tracing")]
            tracing::warn!(file_path = %self.file_path.display(), "Compressed file already exists, saving anyway");
        }

        // Compress the file
        {
            let input_file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.file_path)?;
            let mut reader = BufReader::new(input_file);

            let parent = Path::new(&self.file_path)
                .parent()
                .unwrap_or_else(|| Path::new("."));

            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }

            let tmp_file = NamedTempFile::new_in(parent)?;
            let mut writer = BufWriter::new(&tmp_file);

            lzma_compress(&mut reader, &mut writer)?;

            writer.flush()?;
            drop(writer);

            tmp_file.persist(&compressed_path)?;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            from_file = %self.file_path.display(),
            to_file = %compressed_path.display(),
            "Compressed file",
        );

        // Delete the non-compressed file
        if let Err(_e) = std::fs::remove_file(&self.file_path) {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                file = %self.file_path.display(),
                error = %_e,
                "Failed to remove original file after compression, but compressed file was created successfully"
            );
        }

        self.file_path = compressed_path;
        self.compressed = true;

        Ok(true)
    }

    pub fn decompress(&mut self) -> Result<(), Error> {
        if !self.compressed {
            return Ok(());
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(?self.file_path, "Decompressing file");

        // Define the path for the temporary decompressed file
        let decompressed_path = self.file_path.with_extension(""); // Remove extension

        // Decompress the file
        {
            let input_file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.file_path)?;
            let mut reader = BufReader::new(input_file);

            let parent = Path::new(&decompressed_path)
                .parent()
                .unwrap_or_else(|| Path::new("."));

            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }

            let tmp_file = NamedTempFile::new_in(parent)?;
            let mut writer = BufWriter::new(&tmp_file);

            // Use lzma_decompress to decompress the input file into the output file
            lzma_rs::lzma_decompress(&mut reader, &mut writer)?;

            writer.flush()?;
            drop(writer);

            tmp_file.persist(&decompressed_path)?;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            from_file = %self.file_path.display(),
            to_file = %decompressed_path.display(),
            "Decompressed file",
        );

        // Delete the compressed file
        if let Err(_e) = std::fs::remove_file(&self.file_path) {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                file = %self.file_path.display(),
                error = %_e,
                "failed to remove compressed file after decompression, but decompressed file was created successfully"
            );
        }

        self.file_path = decompressed_path;
        self.compressed = true;

        Ok(())
    }

    /// Load and decompress the data from disk and deserialize it.
    pub fn load<T: BorshDeserialize>(&mut self) -> Result<T, Error> {
        let disk_data = match fs::read(&self.file_path) {
            Ok(data) => data,
            Err(_e) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    file_path = %self.file_path.display(),
                    error = %_e,
                    "Failed to read file, will try again"
                );

                self.detect_file_existence();
                fs::read(&self.file_path)?
            },
        };

        let data = if self.compressed {
            let mut decompressed = Vec::new();
            lzma_decompress(&mut disk_data.as_slice(), &mut decompressed)?;

            decompressed.deserialize_borsh()?
        } else {
            disk_data.deserialize_borsh()?
        };

        Ok(data)
    }

    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }

    pub fn delete(&self) -> Result<(), Error> {
        std::fs::remove_file(&self.file_path)?;
        Ok(())
    }

    pub fn delete_file_path(file_path: &PathBuf) -> Result<(), Error> {
        std::fs::remove_file(file_path)?;
        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, assertor::*};

    #[derive(Debug, BorshSerialize, BorshDeserialize, Default, PartialEq, Eq)]
    struct Block {
        height: u64,
        hash: String,
    }

    #[test]
    fn test_disk_automatic_compression() {
        let temp_file = NamedTempFile::new().expect("failed to create a temp file");
        let mut temp_filename = temp_file.path().to_path_buf();
        let temp_filename2 = temp_file.path().to_path_buf();

        drop(temp_file);

        let block = Block::default();

        let disk_persistence = DiskPersistence::new(temp_filename.clone(), true);
        disk_persistence.save(&block).expect("failed to save block");
        assert!(!temp_filename.exists());

        temp_filename.set_extension("borsh.xz");
        assert!(temp_filename.exists());

        // This test if the file_path is properly set when we ask not to compress
        // but the file already exists compressed.
        let mut disk_persistence = DiskPersistence::new(temp_filename2, false);
        assert_that!(disk_persistence.file_path).is_equal_to(temp_filename);

        // Ensure calling compress on a compressed file doesn't do anything
        assert!(!disk_persistence.compress().expect("can't compress"));
    }

    #[test]
    fn test_disk_later_compression() {
        let temp_file = NamedTempFile::new().expect("failed to create a temp file");
        let mut temp_filename = temp_file.path().to_path_buf();
        let temp_filename2 = temp_file.path().to_path_buf();

        drop(temp_file);

        let block = Block::default();

        let disk_persistence = DiskPersistence::new(temp_filename.clone(), false);
        disk_persistence.save(&block).expect("failed to save block");
        assert!(!temp_filename.exists());

        temp_filename.set_extension("borsh");
        assert!(temp_filename.exists());

        // This test if the file_path is properly set when we ask to compress
        // but the file already exists uncompressed.
        let mut disk_persistence = DiskPersistence::new(temp_filename2, true);
        assert_that!(&disk_persistence.file_path).is_equal_to(&temp_filename);
        assert!(!disk_persistence.compressed);

        // Ensure calling compress on a compressed file does something
        assert!(disk_persistence.compress().expect("can't compress"));

        assert!(
            !temp_filename.exists(),
            "Compressed file should not exist: {}",
            temp_filename.display()
        );

        temp_filename.set_extension("borsh.xz");
        assert!(
            temp_filename.exists(),
            "Compressed file should exist: {}",
            temp_filename.display()
        );

        disk_persistence
            .decompress()
            .expect("Failed to decompress file");

        assert!(
            !temp_filename.exists(),
            "Compressed file should not exist: {}",
            temp_filename.display()
        );

        // This removes the `.xz` extension but leaves the `.borsh` extension
        temp_filename.set_extension("");

        assert!(
            temp_filename.exists(),
            "Decompressed file should exist: {}",
            temp_filename.display()
        );
    }

    #[test]
    fn test_compression_then_read() {
        let temp_file = NamedTempFile::new().expect("failed to create a temp file");
        let mut temp_filename1 = temp_file.path().to_path_buf();
        let temp_filename2 = temp_filename1.clone();

        drop(temp_file);

        let block = Block::default();

        // 1. save the block without compression
        let mut disk_persistence1 = DiskPersistence::new(temp_filename1.clone(), false);
        disk_persistence1.save(&block).expect("failed to save data");
        temp_filename1.set_extension("borsh");
        assert!(temp_filename1.exists());

        let mut disk_persistence2 = DiskPersistence::new(temp_filename2, true);

        // 2. compress the file
        assert!(disk_persistence2.compress().expect("can't compress"));

        // 3. try to read the data back from the uncompressed file
        assert_that!(block).is_equal_to(
            disk_persistence1
                .load::<Block>()
                .expect("failed to load data"),
        );
    }

    #[test]
    fn test_decompression_then_read() {
        let temp_file = NamedTempFile::new().expect("failed to create a temp file");
        let mut temp_filename1 = temp_file.path().to_path_buf();
        let temp_filename2 = temp_filename1.clone();

        drop(temp_file);

        let block = Block::default();

        // 1. save the block without compression
        let disk_persistence1 = DiskPersistence::new(temp_filename1.clone(), false);
        disk_persistence1.save(&block).expect("failed to save data");
        temp_filename1.set_extension("borsh");
        assert!(temp_filename1.exists());

        let mut disk_persistence2 = DiskPersistence::new(temp_filename2, true);

        // 2. compress the file
        assert!(disk_persistence2.compress().expect("can't compress"));

        // 3. open the compressed file and decompress it
        let mut disk_persistence1 = DiskPersistence::new(temp_filename1.clone(), false);
        assert!(disk_persistence1.decompress().is_ok());

        // 3. try to read the data back from the uncompressed file
        assert_that!(block).is_equal_to(
            disk_persistence2
                .load::<Block>()
                .expect("failed to load data"),
        );
    }
}
