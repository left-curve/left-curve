use {
    cw_std::{
        Account, Addr, Binary, BlockInfo, GenesisState, Hash, Item, Map, MockStorage, Query,
        Storage, Tx,
    },
    std::collections::HashMap,
};

pub struct App<'a, S> {
    store:                S,
    chain_id:             String,
    current_block:        Option<BlockInfo>,
    last_finalized_block: Item<'a, BlockInfo>,
    codes:                Map<'a, &'a Hash, Binary>,
    accounts:             Map<'a, &'a Addr, Account>,

    // TODO: these should be a prefixed store based on self.store
    contract_stores: HashMap<Addr, MockStorage>,
}

impl<'a, S> App<'a, S> {
    pub fn new(chain_id: impl Into<String>, store: S) -> Self {
        Self {
            store,
            chain_id:             chain_id.into(),
            current_block:        None,
            last_finalized_block: Item::new("b"),
            codes:                Map::new("c"),
            accounts:             Map::new("a"),
            contract_stores:      HashMap::new(),
        }
    }
}

impl<'a, S: Storage> App<'a, S> {
    pub fn init_chain(&mut self, genesis_state: GenesisState) -> anyhow::Result<()> {
        todo!()
    }

    pub fn finalize_block(&mut self, block: BlockInfo, txs: Vec<Tx>) -> anyhow::Result<()> {
        todo!()
    }

    pub fn commit(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    pub fn query(&self, req: Query) -> anyhow::Result<Binary> {
        todo!()
    }
}
