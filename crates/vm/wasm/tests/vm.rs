use {
    anyhow::ensure,
    grug_app::{App, AppResult},
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_types::{
        to_json_value, Addr, Binary, BlockInfo, Coin, Coins, Config, GenesisState, Hash, Message,
        NumberConst, Permission, Permissions, QueryRequest, QueryResponse, Timestamp, Tx, Uint64,
        GENESIS_SENDER,
    },
    grug_vm_wasm::WasmVm,
    std::{
        collections::{BTreeMap, BTreeSet},
        fs, io, vec,
    },
    tracing_test::traced_test,
};

const ARTIFACTS_DIR: &str = "../../../artifacts";

const MOCK_CHAIN_ID: &str = "grug-1";
const MOCK_SALT: &[u8] = b"bank";
const MOCK_DENOM: &str = "ugrug";
const MOCK_SENDER: Addr = Addr::mock(2);
const MOCK_RECEIVER: Addr = Addr::mock(3);

fn read_wasm_file(filename: &str) -> io::Result<Vec<u8>> {
    fs::read(&format!("{ARTIFACTS_DIR}/{filename}"))
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

    fn execute(&mut self, sender: &Addr, msgs: Vec<Message>) -> AppResult<()> {
        let tx = Tx {
            sender: sender.clone(),
            msgs,
            credential: Binary::empty(),
        };

        // Increment block height and block time
        self.block.height += Uint64::ONE;
        self.block.timestamp = self.block.timestamp.plus_nanos(1);

        // Finalize block + commit
        self.app
            .do_finalize_block(self.block.clone(), vec![(Hash::ZERO, tx)])?;
        self.app.do_commit()
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

    // Load bank contract byte code, and predict its address.
    let bank_code = read_wasm_file("grug_bank-aarch64.wasm")?;
    let bank_code_hash = Hash::from_slice(sha2_256(&bank_code));
    let bank = Addr::compute(&GENESIS_SENDER, &bank_code_hash, MOCK_SALT);

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
                code: bank_code.into(),
            },
            Message::Instantiate {
                code_hash: bank_code_hash,
                msg: to_json_value(&grug_bank::InstantiateMsg {
                    initial_balances: BTreeMap::from([(
                        MOCK_SENDER,
                        Coins::new_one(MOCK_DENOM, 100_u128),
                    )]),
                })?,
                salt: MOCK_SALT.to_vec().into(),
                funds: Coins::new_empty(),
                admin: None,
            },
        ],
    })?;

    // Check that sender has been given 100 ugrug.
    suite.assert_balance(MOCK_SENDER, MOCK_DENOM, 100)?;

    // Sender sends 25 ugrug to the receiver.
    suite.execute(&MOCK_SENDER, vec![Message::Transfer {
        to: MOCK_RECEIVER.clone(),
        coins: vec![Coin::new(MOCK_DENOM, 25_u128)].try_into().unwrap(),
    }])?;

    // Check balances again.
    suite.assert_balance(MOCK_SENDER, MOCK_DENOM, 75)?;
    suite.assert_balance(MOCK_RECEIVER.clone(), MOCK_DENOM, 25)?;

    Ok(())
}
