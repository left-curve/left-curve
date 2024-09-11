#![cfg_attr(rustfmt, rustfmt::skip)]

use crate::{
    Addr, Api, AuthCtx, AuthMode, BlockInfo, Coins, Hash256, ImmutableCtx, MockApi, MockQuerier,
    MockStorage, MutableCtx, Querier, QuerierWrapper, Storage, SudoCtx, Timestamp, Uint64,
};

/// Default mock chain ID used in mock context.
pub const MOCK_CHAIN_ID: &str = "dev-1";

/// Default mock block info used in mock context.
pub const MOCK_BLOCK: BlockInfo = BlockInfo {
    height:    Uint64::new(1),
    timestamp: Timestamp::from_seconds(100),
    hash:      Hash256::ZERO,
};

/// Default contract address used in mock context.
pub const MOCK_CONTRACT: Addr = Addr::mock(0);

pub struct MockContext<S = MockStorage, A = MockApi, Q = MockQuerier> {
    pub storage:  S,
    pub api:      A,
    pub querier:  Q,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Option<Addr>,
    pub funds:    Option<Coins>,
    pub mode:     Option<AuthMode>,
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
            sender:   None,
            funds:    None,
            mode:     None,
        }
    }
}

impl MockContext {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, A, Q> MockContext<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn with_storage<T>(self, storage: T) -> MockContext<T, A, Q> {
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

    pub fn with_api<T>(self, api: T) -> MockContext<S, T, Q> {
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

    pub fn with_querier<T>(self, querier: T) -> MockContext<S, A, T> {
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

    pub fn with_block_height<T>(mut self, height: T) -> Self
    where
        T: Into<Uint64>,
    {
        self.block.height = height.into();
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

    pub fn with_sender(mut self, sender: Addr) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_funds(mut self, funds: Coins) -> Self {
        self.funds = Some(funds);
        self
    }

    pub fn with_mode(mut self, mode: AuthMode) -> Self {
        self.mode = Some(mode);
        self
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

    pub fn set_block_height<T>(mut self, height: T)
    where
        T: Into<Uint64>,
    {
        self.block.height = height.into();
    }

    pub fn set_block_timestamp(mut self, timestamp: Timestamp) {
        self.block.timestamp = timestamp;
    }

    pub fn set_block_hash(mut self, hash: Hash256) {
        self.block.hash = hash;
    }

    pub fn set_contract(&mut self, contract: Addr) {
        self.contract = contract;
    }

    pub fn set_sender(&mut self, sender: Addr) {
        self.sender = Some(sender);
    }

    pub fn set_funds(&mut self, funds: Coins) {
        self.funds = Some(funds);
    }

    pub fn set_mode(&mut self, mode: AuthMode) {
        self.mode = Some(mode);
    }

    pub fn as_immutable(&self) -> ImmutableCtx {
        ImmutableCtx {
            storage:  &self.storage,
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
        }
    }

    pub fn as_mutable(&mut self) -> MutableCtx {
        MutableCtx {
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
            sender:   self.sender(),
            funds:    self.funds(),
            storage:  &mut self.storage,
        }
    }

    pub fn as_sudo(&mut self) -> SudoCtx {
        SudoCtx {
            storage:  &mut self.storage,
            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
        }
    }

    pub fn as_auth(&mut self) -> AuthCtx {
        AuthCtx {

            api:      &self.api,
            querier:  QuerierWrapper::new(&self.querier),
            chain_id: self.chain_id.clone(),
            block:    self.block,
            contract: self.contract,
            mode:     self.mode(),
            storage:  &mut self.storage,
        }
    }

    #[inline]
    pub fn mode(&self) -> AuthMode {
        self.mode.expect("[MockContext]: mode not set")
    }

    #[inline]
    pub fn sender(&self) -> Addr {
        self.sender.expect("[MockContext]: sender not set")
    }

    #[inline]
    pub fn funds(&self) -> Coins {
        self.funds.clone().expect("[MockContext]: funds not set")
    }
}
