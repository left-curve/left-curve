use {
    super::BlockStore,
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    std::{collections::BTreeMap, sync::Mutex},
};

/// In-memory [`BlockStore`] over a `BTreeMap`. For tests and ephemeral runs;
/// production uses the Postgres-backed store.
#[derive(Default)]
pub struct MemoryBlockStore {
    blocks: Mutex<BTreeMap<u64, BlockData>>,
}

#[async_trait]
impl BlockStore for MemoryBlockStore {
    async fn put(&self, height: u64, data: &BlockData) -> AnyResult<()> {
        self.blocks
            .lock()
            .unwrap()
            .entry(height)
            .or_insert_with(|| data.clone());
        Ok(())
    }

    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
        Ok(self.blocks.lock().unwrap().get(&height).cloned())
    }

    async fn max_contiguous(&self, floor: u64) -> AnyResult<Option<u64>> {
        let blocks = self.blocks.lock().unwrap();
        if !blocks.contains_key(&floor) {
            return Ok(None);
        }
        let mut top = floor;
        while blocks.contains_key(&(top + 1)) {
            top += 1;
        }
        Ok(Some(top))
    }

    async fn max_height(&self) -> AnyResult<Option<u64>> {
        Ok(self.blocks.lock().unwrap().keys().next_back().copied())
    }

    async fn gaps(&self, from: u64, to: u64) -> AnyResult<Vec<(u64, u64)>> {
        let blocks = self.blocks.lock().unwrap();
        let mut gaps = Vec::new();
        let mut height = from;
        while height < to {
            if blocks.contains_key(&height) {
                height += 1;
                continue;
            }
            let gap_start = height;
            while height < to && !blocks.contains_key(&height) {
                height += 1;
            }
            gaps.push((gap_start, height - 1));
        }
        Ok(gaps)
    }
}
