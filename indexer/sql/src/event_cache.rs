use {
    grug_types::{Addr, FlatEvent},
    std::{
        collections::{HashMap, VecDeque},
        marker::PhantomData,
        sync::Arc,
    },
    tokio::sync::RwLock,
};

pub struct Writer;
pub struct Reader;

pub struct EventCache<M = Reader> {
    p: PhantomData<M>,
    inner: Arc<RwLock<InnerEventCache>>,
}

struct InnerEventCache {
    window: usize,
    ring: VecDeque<u64>,
    blocks: HashMap<u64, HashMap<Addr, Vec<Arc<FlatEvent>>>>,
}
