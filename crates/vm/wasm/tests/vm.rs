use {
    anyhow::ensure,
    grug_account::{make_sign_bytes, PublicKey, StateResponse},
    grug_app::{App, AppResult},
    grug_crypto::{sha2_256, Identity256},
    grug_db_memory::MemDb,
    grug_types::{
        from_json_value, to_borsh_vec, to_json_value, Addr, BlockInfo, Coin, Coins, Config,
        GenesisState, Hash, Message, NumberConst, Permission, Permissions, QueryRequest,
        QueryResponse, Timestamp, Tx, Uint64, GENESIS_SENDER,
    },
    grug_vm_wasm::WasmVm,
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
    rand::rngs::OsRng,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        fs, io, vec,
    },
    tracing_test::traced_test,
};

const ARTIFACTS_DIR: &str = "../../../artifacts";

const MOCK_CHAIN_ID: &str = "grug-1";
const MOCK_DENOM: &str = "ugrug";
const MOCK_BANK_SALT: &[u8] = b"bank";
const MOCK_SENDER_SALT: &[u8] = b"sender";
const MOCK_RECEIVER_SALT: &[u8] = b"receiver";

fn read_wasm_file(filename: &str) -> io::Result<Vec<u8>> {
    fs::read(format!("{ARTIFACTS_DIR}/{filename}"))
}

struct TestSuite {
    app: App<MemDb, WasmVm>,
    block: BlockInfo,
}

impl TestSuite {
    fn new() -> Self {
        Self {
            app: App::new(MemDb::new()),
            block: BlockInfo {
                height: Uint64::ZERO,
                timestamp: Timestamp::from_nanos(0),
                hash: Hash::ZERO,
            },
        }
    }

    fn init_chain(&mut self, genesis_state: GenesisState) -> AppResult<Hash> {
        self.app
            .do_init_chain(MOCK_CHAIN_ID.to_string(), self.block.clone(), genesis_state)
    }

    fn query(&self, req: QueryRequest) -> AppResult<QueryResponse> {
        self.app.do_query_app(req, self.block.height.into(), false)
    }

    fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
    ) -> anyhow::Result<R> {
        let msg_raw = to_json_value(&msg)?;
        let res_raw = self
            .query(QueryRequest::WasmSmart {
                contract,
                msg: msg_raw,
            })?
            .as_wasm_smart()
            .data;
        Ok(from_json_value(res_raw)?)
    }

    fn query_account_sequence(&self, address: Addr) -> anyhow::Result<u32> {
        self.query_wasm_smart(address, &grug_account::QueryMsg::State {})
            .map(|res: StateResponse| res.sequence)
    }

    fn send_messages(
        &mut self,
        sender: Addr,
        sk: &SigningKey,
        msgs: Vec<Message>,
    ) -> anyhow::Result<()> {
        // Sign the transaction
        let sequence = self.query_account_sequence(sender.clone())?;
        let sign_bytes = make_sign_bytes(sha2_256, &msgs, &sender, MOCK_CHAIN_ID, sequence)?;
        let signature: Signature = sk.sign_digest(Identity256::from(sign_bytes));
        let tx = Tx {
            sender,
            msgs,
            credential: signature.to_vec().into(),
        };

        // Increment block height and block time
        self.block.height += Uint64::ONE;
        self.block.timestamp = self.block.timestamp.plus_nanos(1);

        // Finalize block + commit
        self.app
            .do_finalize_block(self.block.clone(), vec![(Hash::ZERO, tx)])?;
        self.app.do_commit()?;

        Ok(())
    }

    fn assert_balance(&self, address: Addr, denom: &str, expect: u128) -> anyhow::Result<()> {
        let actual = self
            .query(QueryRequest::Balance {
                address,
                denom: denom.to_string(),
            })?
            .as_balance()
            .amount
            .number();

        ensure!(actual == expect);

        Ok(())
    }
}

#[traced_test]
#[test]
fn wasm_vm_works() -> anyhow::Result<()> {
    let mut suite = TestSuite::new();

    // Generate private keys for the accounts
    let sender_sk = SigningKey::random(&mut OsRng);
    let sender_pk = sender_sk.verifying_key().to_encoded_point(true).to_bytes();
    let receiver_sk = SigningKey::random(&mut OsRng);
    let receiver_pk = receiver_sk
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes();

    // Load account contract byte code, and predict account addresses
    let account_code = to_borsh_vec(&read_wasm_file("grug_account-aarch64.wasm")?)?;
    let account_code_hash = Hash::from_slice(sha2_256(&account_code));
    let sender = Addr::compute(&GENESIS_SENDER, &account_code_hash, MOCK_SENDER_SALT);
    let receiver = Addr::compute(&GENESIS_SENDER, &account_code_hash, MOCK_RECEIVER_SALT);

    // Load bank contract byte code, and predict its address.
    let bank_code = to_borsh_vec(&read_wasm_file("grug_bank-aarch64.wasm")?)?;
    let bank_code_hash = Hash::from_slice(sha2_256(&bank_code));
    let bank = Addr::compute(&GENESIS_SENDER, &bank_code_hash, MOCK_BANK_SALT);

    // Genesis the chain. This deploys the bank contract and gives the "sender"
    // account 100 ugrug.
    suite.init_chain(GenesisState {
        config: Config {
            owner: None,
            bank: bank.clone(),
            begin_blockers: vec![],
            end_blockers: vec![],
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
                create_client: Permission::Everybody,
                create_connection: Permission::Everybody,
                create_channel: Permission::Everybody,
            },
            allowed_clients: BTreeSet::new(),
        },
        msgs: vec![
            Message::Upload {
                code: account_code.into(),
            },
            Message::Upload {
                code: bank_code.into(),
            },
            Message::Instantiate {
                code_hash: account_code_hash.clone(),
                msg: to_json_value(&grug_account::InstantiateMsg {
                    public_key: PublicKey::Secp256k1(sender_pk.to_vec().into()),
                })?,
                salt: MOCK_SENDER_SALT.to_vec().into(),
                funds: Coins::new_empty(),
                admin: Some(sender.clone()),
            },
            Message::Instantiate {
                code_hash: account_code_hash.clone(),
                msg: to_json_value(&grug_account::InstantiateMsg {
                    public_key: PublicKey::Secp256k1(receiver_pk.to_vec().into()),
                })?,
                salt: MOCK_SENDER_SALT.to_vec().into(),
                funds: Coins::new_empty(),
                admin: Some(receiver.clone()),
            },
            Message::Instantiate {
                code_hash: bank_code_hash,
                msg: to_json_value(&grug_bank::InstantiateMsg {
                    initial_balances: BTreeMap::from([(
                        sender.clone(),
                        Coins::new_one(MOCK_DENOM, 100_u128),
                    )]),
                })?,
                salt: MOCK_BANK_SALT.to_vec().into(),
                funds: Coins::new_empty(),
                admin: None,
            },
        ],
    })?;

    // Check that sender has been given 100 ugrug.
    suite.assert_balance(sender.clone(), MOCK_DENOM, 100)?;

    // Sender sends 25 ugrug to the receiver.
    suite.send_messages(sender.clone(), &sender_sk, vec![Message::Transfer {
        to: receiver.clone(),
        coins: vec![Coin::new(MOCK_DENOM, 25_u128)].try_into().unwrap(),
    }])?;

    // Check balances again.
    suite.assert_balance(sender, MOCK_DENOM, 75)?;
    suite.assert_balance(receiver, MOCK_DENOM, 25)?;

    Ok(())
}
