use {
    cw_db::MockStorage,
    cw_std::{Account, Addr, Binary, Hash},
    std::collections::HashMap,
};

pub struct Block {
    pub height:    u64,
    pub timestamp: u64,
}

pub struct StateMachine {
    // TODO: these should be prefixed stored
    pub contract_stores: HashMap<Addr, MockStorage>,

    // updated by ABCI FinalizeBlock call
    pub current_block: Option<Block>,

    // TODO: these should be part of the chain state
    pub chain_id:             String,
    pub last_finalized_block: Block,
    pub codes:                HashMap<Hash, Binary>,
    pub accounts:             HashMap<Addr, Account>,
}

impl StateMachine {
    pub fn new_mock() -> Self {
        Self {
            contract_stores:      HashMap::new(),
            current_block:        None,
            chain_id:             "dev-1".into(),
            last_finalized_block: Block { height: 1, timestamp: 1 },
            codes:                HashMap::new(),
            accounts:             HashMap::new(),
        }
    }
}
