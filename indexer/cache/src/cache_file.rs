use {
    super::error,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Block, BlockAndBlockOutcomeWithHttpDetails, BlockOutcome},
    indexer_disk_saver::persistence::DiskPersistence,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        ops::{Deref, DerefMut},
        path::PathBuf,
    },
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct CacheFile {
    #[serde(flatten)]
    pub data: BlockAndBlockOutcomeWithHttpDetails,
    #[serde(skip)]
    #[borsh(skip)]
    filename: PathBuf,
}

impl Deref for CacheFile {
    type Target = BlockAndBlockOutcomeWithHttpDetails;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for CacheFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl CacheFile {
    pub fn new(filename: PathBuf, block: Block, block_outcome: BlockOutcome) -> Self {
        Self {
            data: BlockAndBlockOutcomeWithHttpDetails {
                block,
                block_outcome,
                http_request_details: HashMap::new(),
            },
            filename,
        }
    }

    pub fn save_to_disk(&self) -> error::Result<()> {
        Ok(DiskPersistence::new(self.filename.clone(), false).save(self)?)
    }

    pub fn compress_file(file_path: PathBuf) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(?file_path, "Compressing block file");

        let mut file = DiskPersistence::new(file_path.clone(), false);

        #[allow(unused_variables)]
        let compressed = file.compress()?;

        #[cfg(feature = "tracing")]
        tracing::info!(
            ?file.file_path,
            ?compressed,
            "Compressed block file"
        );

        Ok(())
    }

    pub fn load_from_disk(file_path: PathBuf) -> error::Result<Self> {
        let mut block_to_index: Self = DiskPersistence::new(file_path.clone(), false).load()?;
        block_to_index.filename = file_path;
        Ok(block_to_index)
    }

    pub async fn load_from_disk_async(file_path: PathBuf) -> error::Result<Self> {
        tokio::task::spawn_blocking(move || Self::load_from_disk(file_path)).await?
    }

    pub fn exists(file_path: PathBuf) -> bool {
        DiskPersistence::new(file_path, false).exists()
    }

    pub fn delete_from_disk(file_path: PathBuf) -> error::Result<()> {
        Ok(DiskPersistence::new(file_path, false).delete()?)
    }
}

// I might need to implement those manually one day, if changing the structs
// breaks compatibility.
// impl BorshSerialize for CacheFile {
//     fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
//         BorshSerialize::serialize(&self.data.block, writer)?;
//         BorshSerialize::serialize(&self.data.block_outcome, writer)?;
//         let string_map: HashMap<String, _> = self
//             .data
//             .http_request_details
//             .iter()
//             .map(|(k, v)| (k.to_string(), v.clone()))
//             .collect();
//         BorshSerialize::serialize(&string_map, writer)?;
//         Ok(())
//     }
// }

// impl BorshDeserialize for CacheFile {
//     fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
//         let block = Block::deserialize_reader(reader)?;
//         let block_outcome = BlockOutcome::deserialize_reader(reader)?;
//         let string_map: HashMap<String, HttpRequestDetails> =
//             BorshDeserialize::deserialize_reader(reader)?;

//         Ok(Self {
//             data: BlockAndBlockOutcomeWithHttpDetails {
//                 block,
//                 block_outcome,
//                 http_request_details: string_map
//                     .into_iter()
//                     .map(|(k, v)| {
//                         Hash256::from_str(&k)
//                             .map(|hash| (hash, v))
//                             .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
//                     })
//                     .collect::<std::io::Result<_>>()?,
//             },
//             filename: PathBuf::new(),
//         })
//     }
// }

#[cfg(test)]
mod tests {
    use {
        super::*,
        assertor::*,
        grug_types::{BlockInfo, Hash},
        tempfile::NamedTempFile,
    };

    #[test]
    fn test_save_and_load_and_delete_tmp_file() {
        let block_info = BlockInfo {
            height: 10,
            timestamp: Default::default(),
            hash: Hash::ZERO,
        };

        let block = Block {
            info: block_info,
            txs: vec![],
        };

        let block_outcome = BlockOutcome {
            height: 10,
            app_hash: Hash::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        let temp_file = NamedTempFile::new().expect("Failed to create a temp file");
        let temp_filename = temp_file.path().to_path_buf();

        let block_to_index = CacheFile::new(temp_filename.clone(), block, block_outcome);

        assert_that!(block_to_index.save_to_disk()).is_ok();

        let saved_block_to_index =
            CacheFile::load_from_disk(temp_filename.clone()).expect("Can't load tmp file");

        assert_that!(saved_block_to_index).is_equal_to(block_to_index);
        assert_that!(CacheFile::delete_from_disk(temp_filename)).is_ok();
    }
}
