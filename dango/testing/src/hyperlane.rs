use {
    crate::generate_random_key,
    dango_types::{
        config::{AppAddresses, AppConfig},
        warp::{self, QueryRouteRequest, Route, TokenMessage},
    },
    grug::{
        Addr, Coin, Coins, Denom, Hash256, HashExt, HexBinary, HexByteArray, Inner, NumberConst,
        QuerierExt, ResultExt, Signer, TestSuite, TxOutcome, Uint128,
    },
    grug_app::{AppError, Db, Indexer, ProposalPreparer, Shared, Vm},
    grug_crypto::Identity256,
    hyperlane_types::{
        addr32, domain_hash, eip191_hash,
        isms::{self, multisig::Metadata, HYPERLANE_DOMAIN_KEY},
        mailbox::{self, Domain, Message, MAILBOX_VERSION},
        multisig_hash, Addr32, IncrementalMerkleTree,
    },
    k256::ecdsa::SigningKey,
    std::{
        collections::BTreeSet,
        ops::{Deref, DerefMut},
    },
};

const MOCK_REMOTE_MERKLE_TREE: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000002");

const MOCK_REMOTE_DOMAIN: Domain = 123;

const MOCK_LOCAL_DOMAIN: Domain = 88888888;

const MOCK_REMOTE_ROUTE: Route = Route {
    address: addr32!("0000000000000000000000000000000000000000000000000000000000000000"),
    fee: Uint128::ZERO,
};

pub struct HyperlaneTestSuite<DB, VM, PP, ID, O>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    suite: TestSuite<DB, VM, PP, ID>,
    val_set: MockValidatorSet,
    owner: Shared<O>,
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
        size: usize,
        threshold: usize,
        remote_domain: Domain,
    ) -> (Self, Shared<O>) {
        let mock_validator_set = MockValidatorSet::new_random(size);

        let ism = suite
            .query_app_config::<AppConfig>()
            .should_succeed()
            .addresses
            .ism;

        // Set validators at the ISM.
        suite
            .execute(
                &mut owner,
                ism,
                &isms::multisig::ExecuteMsg::SetValidators {
                    domain: remote_domain,
                    threshold: threshold as u32,
                    validators: mock_validator_set.addresses.clone(),
                },
                Coins::new(),
            )
            .should_succeed();

        let owner = Shared::new(owner);

        (
            Self {
                suite,
                val_set: mock_validator_set,
                owner: owner.clone(),
            },
            owner,
        )
    }

    pub fn new_mocked(suite: TestSuite<DB, VM, PP, ID>, owner: O) -> (Self, Shared<O>) {
        Self::new(suite, owner, 3, 2, MOCK_REMOTE_DOMAIN)
    }

    pub fn hyperlane(&mut self) -> HyperlaneHelper<DB, VM, PP, ID, O> {
        HyperlaneHelper { suite: self }
    }
}

impl<DB, VM, PP, ID, O> Deref for HyperlaneTestSuite<DB, VM, PP, ID, O>
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

impl<'a, DB, VM, PP, ID, O> DerefMut for HyperlaneTestSuite<DB, VM, PP, ID, O>
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

pub struct HyperlaneHelper<'a, DB, VM, PP, ID, O>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    O: Signer,
{
    suite: &'a mut HyperlaneTestSuite<DB, VM, PP, ID, O>,
}

impl<'a, DB, VM, PP, ID, O> HyperlaneHelper<'a, DB, VM, PP, ID, O>
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
        );

        let shared_owner = self.suite.owner.clone();
        let mut owner = shared_owner.write_access();

        self.suite
            .execute(
                owner.deref_mut(),
                addresses.mailbox,
                &mailbox::ExecuteMsg::Process {
                    raw_message: raw_message.clone(),
                    raw_metadata: raw_metadata.clone(),
                },
                Coins::new(),
            )
            .should_succeed();

        ReceiveTransferResponse::new(raw_metadata, raw_message)
    }

    pub fn recieve_transfer_mock(&mut self, to: Addr, coin: Coin) {
        let addresses = self.addresses();

        if self
            .suite
            .query_wasm_smart(addresses.warp, QueryRouteRequest {
                denom: coin.denom.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
            })
            .is_err()
        {
            self.set_route(coin.denom.clone(), MOCK_REMOTE_DOMAIN, MOCK_REMOTE_ROUTE)
                .should_succeed();
        }

        self.receive_transfer(MOCK_REMOTE_DOMAIN, to, coin);
    }

    pub fn send_transfer(
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

    pub fn set_route(
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

    fn create_msg(
        &mut self,
        origin_domain: Domain,
        sender: Addr32,
        recipient: Addr32,
        body: HexBinary,
    ) -> (HexBinary, HexBinary) {
        let raw_message = Message {
            version: MAILBOX_VERSION,
            nonce: 0,
            origin_domain,
            sender,
            destination_domain: MOCK_LOCAL_DOMAIN, // this should be our local domain
            recipient,
            body,
        }
        .encode();

        let message_id = raw_message.keccak256();
        let raw_metadata = self.suite.val_set.sign(message_id).encode();

        (raw_message, raw_metadata)
    }

    fn addresses(&self) -> AppAddresses {
        self.suite
            .query_app_config::<AppConfig>()
            .should_succeed()
            .addresses
    }
}

struct MockValidatorSet {
    secrets: Vec<SigningKey>,
    addresses: BTreeSet<HexByteArray<20>>,
    merkle_tree: IncrementalMerkleTree,
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
            merkle_tree: IncrementalMerkleTree::default(),
        }
    }

    pub fn sign(&mut self, message_id: Hash256) -> Metadata {
        self.merkle_tree.insert(message_id).unwrap();

        let merkle_root = self.merkle_tree.root();
        let merkle_index = (self.merkle_tree.count - 1) as u32;

        let multisig_hash = eip191_hash(multisig_hash(
            domain_hash(
                MOCK_REMOTE_DOMAIN,
                MOCK_REMOTE_MERKLE_TREE,
                HYPERLANE_DOMAIN_KEY,
            ),
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
