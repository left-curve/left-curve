use {
    crate::{Cache, error, indexer_path::IndexerPath},
    grug_types::{Defined, MaybeDefined, Undefined},
    std::path::PathBuf,
};

pub struct IndexerBuilder<P = Undefined<IndexerPath>> {
    indexer_path: P,
}

impl IndexerBuilder<Undefined<IndexerPath>> {
    pub fn with_tmpdir(self) -> IndexerBuilder<Defined<IndexerPath>> {
        IndexerBuilder {
            indexer_path: Defined::new(IndexerPath::default()),
        }
    }

    pub fn with_dir(self, dir: PathBuf) -> IndexerBuilder<Defined<IndexerPath>> {
        IndexerBuilder {
            indexer_path: Defined::new(IndexerPath::Dir(dir)),
        }
    }

    pub fn build(self) -> error::Result<Cache> {
        let indexer_path = self.indexer_path.maybe_into_inner().unwrap_or_default();
        indexer_path.create_dirs_if_needed()?;

        Ok(Cache::new(indexer_path))
    }
}
