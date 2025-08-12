use {
    crate::entity::events::Model as EventModel,
    grug_types::Addr,
    std::{
        collections::{BTreeSet, HashMap},
        marker::PhantomData,
        ops::{Deref, RangeInclusive},
        sync::Arc,
    },
    tokio::sync::RwLock,
};

pub type EventCacheWriter = EventCache<Writer>;
pub type EventCacheReader = EventCache<Reader>;

#[derive(Clone)]
pub struct Writer;

#[derive(Clone)]
pub struct Reader;

struct InnerEventCache {
    window: usize,
    ring: BTreeSet<u64>,
    blocks: HashMap<u64, HashMap<Addr, Vec<Arc<EventModel>>>>,
}

#[derive(Clone)]
pub struct EventCache<M = Reader> {
    p: PhantomData<M>,
    inner: Arc<RwLock<InnerEventCache>>,
}

impl<M> EventCache<M> {
    pub fn new(block_window: usize) -> EventCache<M> {
        Self {
            p: PhantomData,
            inner: Arc::new(RwLock::new(InnerEventCache {
                window: block_window,
                ring: BTreeSet::new(),
                blocks: HashMap::new(),
            })),
        }
    }

    pub async fn read_events(
        &self,
        block_range: RangeInclusive<u64>,
        addresses: &[Addr],
    ) -> Vec<EventModel> {
        // use Vec<Arc<EventModel>> instead of Vec<EventModel> in order to release the lock as soon as possible
        let events = {
            let inner = self.inner.read().await;

            let mut events = vec![];

            let (Some(lower_bound), Some(upper_bound)) = (inner.ring.first(), inner.ring.last())
            else {
                return vec![];
            };

            let block_range = {
                let range_low = block_range.start().max(lower_bound);
                let range_high = block_range.end().min(upper_bound);
                *range_low..=*range_high
            };

            for block in block_range {
                let Some(block_events) = inner.blocks.get(&block) else {
                    continue;
                };

                for addr in addresses {
                    let Some(evt) = block_events.get(addr) else {
                        continue;
                    };

                    events.extend(evt.clone());
                }
            }

            events
        };

        events.into_iter().map(|e| e.deref().clone()).collect()
    }
}

impl EventCacheWriter {
    pub async fn save_events(&self, block: u64, map: HashMap<Addr, Vec<Arc<EventModel>>>) {
        let mut inner = self.inner.write().await;

        // If the block is already in the ring it get overwritten
        // This should never happen

        inner.blocks.insert(block, map);
        inner.ring.insert(block);
        if inner.ring.len() > inner.window {
            let Some(to_remove) = inner.ring.pop_first() else {
                return;
            };
            inner.blocks.remove(&to_remove);
        }
    }

    pub fn as_reader(&self) -> EventCacheReader {
        EventCacheReader {
            p: PhantomData,
            inner: self.inner.clone(),
        }
    }
}
