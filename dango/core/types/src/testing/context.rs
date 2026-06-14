#![cfg_attr(rustfmt, rustfmt::skip)]

use crate::{
    Addr, Api, AuthCtx, AuthMode, BlockInfo, Coins, Defined, Hash256, ImmutableCtx, MockApi,
    MockQuerier, MockStorage, MutableCtx, Querier, QuerierWrapper, Storage, SudoCtx, Timestamp,
    Undefined,
};

/// Default mock chain ID used in mock context.
pub const MOCK_CHAIN_ID: &str = "dev-1";

/// Default mock block info used in mock context.
pub const MOCK_BLOCK: BlockInfo = BlockInfo {
    height:    1,
    timestamp: Timestamp::from_seconds(100),
    hash:      Hash256::ZERO,
};

/// Default contract address used in mock context.
pub const MOCK_CONTRACT: Addr = Addr::mock(0);

/// A mock up context for use in unit tests.
pub struct MockContext<
    S = MockStorage,
    A = MockApi,
    Q = MockQuerier,
    E = Undefined<Addr>,
    F = Undefined<Coins>,
    M = Undefined<AuthMode>,
> {
    pub storage:  S,
    pub api:      A,
    pub querier:  Q,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   E,
    pub funds:    F,
    pub mode:     M,
}

impl Default for MockContext {
    fn default() -> Self {
        Self {
            storage:  MockStorage::new(),
            api:      MockApi,
            querier:  MockQuerier::new(),
            chain_id: MOCK_CHAIN_ID.to_string(),
            block:    MOCK_BLOCK,
            contract: MOCK_CONTRACT,
            sender:   Undefined::new(),
            funds:    Undefined::new(),
            mode:     Undefined::new(),
        }
    }
}

impl MockContext {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, A, Q, E, F, M> MockContext<S, A, Q, E, F, M> {
    pub fn with_storage<T>(self, storage: T) -> MockContext<T, A, Q, E, F, M> {
        MockContext {
            storage,
            api:      self.api,
            querier:  self.querier,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   self.sender,
            funds:    self.funds,
            mode:     self.mode,
        }
    }

    pub fn with_api<T>(self, api: T) -> MockContext<S, T, Q, E, F, M> {
        MockContext {
            api,
            storage:  self.storage,
            querier:  self.querier,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   self.sender,
            funds:    self.funds,
            mode:     self.mode,
        }
    }

    pub fn with_querier<T>(self, querier: T) -> MockContext<S, A, T, E, F, M> {
        MockContext {
            querier,
            storage:  self.storage,
            api:      self.api,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   self.sender,
            funds:    self.funds,
            mode:     self.mode,
        }
    }

    pub fn with_sender(self, sender: Addr) -> MockContext<S, A, Q, Defined<Addr>, F, M> {
        MockContext {
            storage:  self.storage,
            api:      self.api,
            querier:  self.querier,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   Defined::new(sender),
            funds:    self.funds,
            mode:     self.mode,
        }
    }

    pub fn with_funds(self, funds: Coins) -> MockContext<S, A, Q, E, Defined<Coins>, M> {
        MockContext {
            storage:  self.storage,
            api:      self.api,
            querier:  self.querier,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   self.sender,
            funds:    Defined::new(funds),
            mode:     self.mode,
        }
    }

    pub fn with_mode(self, mode: AuthMode) -> MockContext<S, A, Q, E, F, Defined<AuthMode>> {
        MockContext {
            storage:  self.storage,
            api:      self.api,
            querier:  self.querier,
            chain_id: self.chain_id,
            block:    self.block,
            contract: self.contract,
            sender:   self.sender,
            funds:    self.funds,
            mode:     Defined::new(mode),
        }
    }

    pub fn with_chain_id<T>(mut self, chain_id: T) -> Self
    where
        T: Into<String>,
    {
        self.chain_id = chain_id.into();
        self
    }

    pub fn with_block(mut self, block: BlockInfo) -> Self {
        self.block = block;
        self
    }

    pub fn with_block_height(mut self, height: u64) -> Self {
        self.block.height = height;
        self
    }

    pub fn with_block_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.block.timestamp = timestamp;
        self
    }

    pub fn with_block_hash(mut self, hash: Hash256) -> Self {
        self.block.hash = hash;
        self
    }

    pub fn with_contract(mut self, contract: Addr) -> Self {
        self.contract = contract;
        self
    }

    pub fn update_querier<C>(&mut self, callback: C)
    where
        C: FnOnce(&mut Q),
    {
        callback(&mut self.querier);
    }

    pub fn set_chain_id<T>(&mut self, chain_id: T)
    where
        T: Into<String>,
    {
        self.chain_id = chain_id.into();
    }

    pub fn set_block(&mut self, block: BlockInfo) {
        self.block = block;
    }

    pub fn set_block_height(&mut self, height: u64) {
        self.block.height = height;
    }

    pub fn set_block_timestamp(&mut self, timestamp: Timestamp) {
        self.block.timestamp = timestamp;
    }

    pub fn set_block_hash(&mut self, hash: Hash256) {
        self.block.hash = hash;
    }

    pub fn set_contract(&mut self, contract: Addr) {
        self.contract = contract;
    }
}

impl<S, A, Q> MockContext<S, A, Q, Undefined<Addr>, Undefined<Coins>, Undefined<AuthMode>>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn as_immutable(&self) -> ImmutableCtx<'_> {
        ImmutableCtx {
            storage:  &self.storage,
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
        }
    }

    pub fn as_sudo(&mut self) -> SudoCtx<'_> {
        SudoCtx {
            storage:  &mut self.storage,
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
        }
    }
}

impl<S, A, Q> MockContext<S, A, Q, Defined<Addr>, Defined<Coins>, Undefined<AuthMode>>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn as_mutable(&mut self) -> MutableCtx<'_> {
        MutableCtx {
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
            sender:   self.sender.into_inner(),
            funds:    self.funds.clone().into_inner(),
            storage:  &mut self.storage,
        }
    }
}

impl<S, A, Q> MockContext<S, A, Q, Undefined<Addr>, Undefined<Coins>, Defined<AuthMode>>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn as_auth(&mut self) -> AuthCtx<'_> {
        AuthCtx {
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
            mode:     self.mode.into_inner(),
            storage:  &mut self.storage,
        }
    }
}
