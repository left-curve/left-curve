use {
    crate::generate_random_key,
    dango_types::{
        config::{AppAddresses, AppConfig},
        warp::{self, QueryRouteRequest, Route, TokenMessage},
    },
    grug::{
        Addr, Coin, Coins, Denom, Hash256, HashExt, HexBinary, HexByteArray, Inner, NumberConst,
        QuerierExt, ResultExt, Signer, TestSuite, TxOutcome, Uint128, btree_map,
    },
    grug_app::{AppError, Db, Indexer, ProposalPreparer, Shared, Vm},
    grug_crypto::Identity256,
    hyperlane_types::{
        Addr32, IncrementalMerkleTree, addr32, domain_hash, eip191_hash,
        isms::{self, HYPERLANE_DOMAIN_KEY, multisig::Metadata},
        mailbox::{self, Domain, MAILBOX_VERSION, Message},
        multisig_hash,
    },
    k256::ecdsa::SigningKey,
    std::{
        collections::{BTreeMap, BTreeSet},
        ops::{Deref, DerefMut},
    },
};

pub const MOCK_LOCAL_DOMAIN: Domain = 88888888;

pub const MOCK_REMOTE_DOMAIN: Domain = 123;

pub const MOCK_REMOTE_MERKLE_TREE: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000002");

pub const MOCK_REMOTE_ROUTE: Route = Route {
    address: addr32!("0000000000000000000000000000000000000000000000000000000000000000"),
    fee: Uint128::ZERO,
};

pub struct HyperlaneTestSuite<DB, VM, PP, ID, O, VS = BTreeMap<Domain, MockValidatorSet>>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    suite: TestSuite<DB, VM, PP, ID>,
    owner: Shared<O>,
    validator_sets: VS,
}

impl<DB, VM, PP, ID, O> HyperlaneTestSuite<DB, VM, PP, ID, O>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
    O: Signer,
{
    pub fn new(
        mut suite: TestSuite<DB, VM, PP, ID>,
        mut owner: O,
        validator_sets: BTreeMap<Domain, (usize, usize)>,
    ) -> (HyperlaneTestSuite<DB, VM, PP, ID, O>, Shared<O>) {
        let ism = suite
            .query_app_config::<AppConfig>()
            .should_succeed()
            .addresses
            .hyperlane
            .ism;

        let validator_sets = validator_sets
            .into_iter()
            .map(|(domain, (size, threshold))| {
                let mock_validator_set = MockValidatorSet::new_random(size);

                // Set validators at the ISM.
                suite
                    .execute(
                        &mut owner,
                        ism,
                        &isms::multisig::ExecuteMsg::SetValidators {
                            domain,
                            threshold: threshold as u32,
                            validators: mock_validator_set.addresses.clone(),
                        },
                        Coins::new(),
                    )
                    .should_succeed();

                (domain, mock_validator_set)
            })
            .collect();

        let owner = Shared::new(owner);

        (
            HyperlaneTestSuite {
                suite,
                owner: owner.clone(),
                validator_sets,
            },
            owner,
        )
    }

    /// Create a new mocked HyperlaneTestSuite.
    ///
    /// The mocked version use mocked domain, simplifying the call of the functions
    /// where is tested only vs a single domain.
    pub fn new_mocked(
        suite: TestSuite<DB, VM, PP, ID>,
        owner: O,
    ) -> (
        HyperlaneTestSuite<DB, VM, PP, ID, O, MockValidatorSet>,
        Shared<O>,
    ) {
        let (mut suite, owner) =
            Self::new(suite, owner, btree_map! { MOCK_REMOTE_DOMAIN => (3, 2) });

        let suite = HyperlaneTestSuite {
            suite: suite.suite,
            owner: suite.owner,
            validator_sets: suite.validator_sets.pop_last().unwrap().1,
        };

        (suite, owner)
    }
}

impl<DB, VM, PP, ID, O, VS> HyperlaneTestSuite<DB, VM, PP, ID, O, VS>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
    O: Signer,
{
    pub fn hyperlane(&mut self) -> HyperlaneHelper<DB, VM, PP, ID, O, VS> {
        HyperlaneHelper { suite: self }
    }
}

impl<DB, VM, PP, ID, O, VS> Deref for HyperlaneTestSuite<DB, VM, PP, ID, O, VS>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    type Target = TestSuite<DB, VM, PP, ID>;

    fn deref(&self) -> &Self::Target {
        &self.suite
    }
}

impl<DB, VM, PP, ID, O, VS> DerefMut for HyperlaneTestSuite<DB, VM, PP, ID, O, VS>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.suite
    }
}

pub struct HyperlaneHelper<'a, DB, VM, PP, ID, O, VS = BTreeMap<Domain, MockValidatorSet>>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    suite: &'a mut HyperlaneTestSuite<DB, VM, PP, ID, O, VS>,
}

// ----------------------------------- Shared ----------------------------------

impl<DB, VM, PP, ID, O, VS> HyperlaneHelper<'_, DB, VM, PP, ID, O, VS>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn addresses(&self) -> AppAddresses {
        self.suite
            .query_app_config::<AppConfig>()
            .should_succeed()
            .addresses
    }

    fn create_msg(
        &self,
        origin_domain: Domain,
        sender: Addr32,
        recipient: Addr32,
        body: HexBinary,
        validator_set: MockValidatorSet,
    ) -> (HexBinary, HexBinary) {
        let raw_message = Message {
            version: MAILBOX_VERSION,
            nonce: validator_set.next_nonce(),
            origin_domain,
            sender,
            destination_domain: MOCK_LOCAL_DOMAIN, // this should be our local domain
            recipient,
            body,
        }
        .encode();

        let message_id = raw_message.keccak256();
        let raw_metadata = validator_set.sign(message_id, origin_domain).encode();

        (raw_message, raw_metadata)
    }

    fn do_receive_transfer(
        &mut self,
        domain: Domain,
        to: Addr,
        coin: Coin,
        validator_set: MockValidatorSet,
    ) -> ReceiveTransferResponse {
        let addresses = self.addresses();

        let route = self
            .suite
            .query_wasm_smart(addresses.warp, QueryRouteRequest {
                denom: coin.denom,
                destination_domain: domain,
            })
            .should_succeed();

        let (raw_message, raw_metadata) = self.create_msg(
            domain,
            route.address,
            addresses.warp.into(),
            TokenMessage {
                recipient: to.into(),
                amount: coin.amount,
                metadata: HexBinary::default(),
            }
            .encode(),
            validator_set,
        );

        let shared_owner = self.suite.owner.clone();
        let mut owner = shared_owner.write_access();

        self.suite
            .execute(
                owner.deref_mut(),
                addresses.hyperlane.mailbox,
                &mailbox::ExecuteMsg::Process {
                    raw_message: raw_message.clone(),
                    raw_metadata: raw_metadata.clone(),
                },
                Coins::new(),
            )
            .should_succeed();

        ReceiveTransferResponse::new(raw_metadata, raw_message)
    }

    fn do_send_transfer(
        &mut self,
        sender: &mut dyn Signer,
        domain: Domain,
        to: Addr32,
        coin: Coin,
    ) -> TxOutcome {
        let warp_addr = self.addresses().warp;
        self.suite.execute(
            sender,
            warp_addr,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: domain,
                recipient: to,
                metadata: None,
            },
            coin,
        )
    }

    fn do_set_route(
        &mut self,
        denom: Denom,
        destination_domain: Domain,
        route: Route,
    ) -> TxOutcome {
        let warp_addr = self
            .suite
            .query_app_config::<AppConfig>()
            .should_succeed()
            .addresses
            .warp;

        let shared_owner = self.suite.owner.clone();
        let mut owner = shared_owner.write_access();

        self.suite.execute(
            owner.deref_mut(),
            warp_addr,
            &warp::ExecuteMsg::SetRoute {
                denom,
                destination_domain,
                route,
            },
            Coins::new(),
        )
    }
}

// -------------------------------- Non mocked ---------------------------------

impl<DB, VM, PP, ID, O> HyperlaneHelper<'_, DB, VM, PP, ID, O>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    pub fn receive_transfer(
        &mut self,
        domain: Domain,
        to: Addr,
        coin: Coin,
    ) -> ReceiveTransferResponse {
        let validator_set = self.suite.validator_sets.get(&domain).unwrap().clone();
        self.do_receive_transfer(domain, to, coin, validator_set)
    }

    pub fn send_transfer(
        &mut self,
        sender: &mut dyn Signer,
        domain: Domain,
        to: Addr32,
        coin: Coin,
    ) -> TxOutcome {
        self.do_send_transfer(sender, domain, to, coin)
    }

    pub fn set_route(
        &mut self,
        denom: Denom,
        destination_domain: Domain,
        route: Route,
    ) -> TxOutcome {
        self.do_set_route(denom, destination_domain, route)
    }
}

// ---------------------------------- Mocked -----------------------------------

impl<DB, VM, PP, ID, O> HyperlaneHelper<'_, DB, VM, PP, ID, O, MockValidatorSet>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    pub fn receive_transfer(&mut self, to: Addr, coin: Coin) {
        let addresses = self.addresses();

        if self
            .suite
            .query_wasm_smart(addresses.warp, QueryRouteRequest {
                denom: coin.denom.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
            })
            .is_err()
        {
            self.do_set_route(coin.denom.clone(), MOCK_REMOTE_DOMAIN, MOCK_REMOTE_ROUTE)
                .should_succeed();
        }

        self.do_receive_transfer(
            MOCK_REMOTE_DOMAIN,
            to,
            coin,
            self.suite.validator_sets.clone(),
        );
    }

    pub fn send_transfer(&mut self, sender: &mut dyn Signer, to: Addr32, coin: Coin) -> TxOutcome {
        self.do_send_transfer(sender, MOCK_REMOTE_DOMAIN, to, coin)
    }
}

#[derive(Clone)]
pub struct MockValidatorSet {
    secrets: Vec<SigningKey>,
    addresses: BTreeSet<HexByteArray<20>>,
    merkle_tree: Shared<IncrementalMerkleTree>,
    nonce: Shared<u32>,
}

impl MockValidatorSet {
    pub fn new_random(size: usize) -> Self {
        let (secrets, addresses) = (0..size)
            .map(|_| {
                let (sk, _) = generate_random_key();
                // We need the _uncompressed_ pubkey for deriving Ethereum address.
                let pk = sk.verifying_key().to_encoded_point(false).to_bytes();
                let pk_hash = (&pk[1..]).keccak256();
                let address = &pk_hash[12..];

                (sk, HexByteArray::from_inner(address.try_into().unwrap()))
            })
            .unzip();

        Self {
            secrets,
            addresses,
            merkle_tree: Shared::new(IncrementalMerkleTree::default()),
            nonce: Shared::new(0),
        }
    }

    pub fn sign(&self, message_id: Hash256, origin_domain: Domain) -> Metadata {
        self.merkle_tree.write_access().insert(message_id).unwrap();

        let merkle_root = self.merkle_tree.read_access().root();
        let merkle_index = (self.merkle_tree.read_access().count - 1) as u32;

        let multisig_hash = eip191_hash(multisig_hash(
            domain_hash(origin_domain, MOCK_REMOTE_MERKLE_TREE, HYPERLANE_DOMAIN_KEY),
            merkle_root,
            merkle_index,
            message_id,
        ));

        let signatures = self
            .secrets
            .iter()
            .map(|sk| {
                let (signature, recovery_id) = sk
                    .sign_digest_recoverable(Identity256::from(multisig_hash.into_inner()))
                    .unwrap();

                let mut packed = [0u8; 65];
                packed[..64].copy_from_slice(&signature.to_bytes());
                packed[64] = recovery_id.to_byte() + 27;
                HexByteArray::from_inner(packed)
            })
            .collect();

        Metadata {
            origin_merkle_tree: MOCK_REMOTE_MERKLE_TREE,
            merkle_root,
            merkle_index,
            signatures,
        }
    }

    pub fn next_nonce(&self) -> u32 {
        *self.nonce.write_access() += 1;
        *self.nonce.read_access()
    }
}

pub struct ReceiveTransferResponse {
    pub message_id: Hash256,
    pub raw_metadata: HexBinary,
    pub raw_message: HexBinary,
}

impl ReceiveTransferResponse {
    pub fn new(raw_metadata: HexBinary, raw_message: HexBinary) -> Self {
        Self {
            message_id: raw_message.keccak256(),
            raw_metadata,
            raw_message,
        }
    }
}
