use {
    grug_app::{App, AppResult},
    grug_bank::InstantiateMsg,
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_testing::current_time,
    grug_types::{
        to_borsh_vec, to_json_value, Addr, Binary, BlockInfo, Coin, Coins, Config, Event,
        GenesisState, Hash, Message, NumberConst, Permission, Permissions, QueryRequest,
        QueryResponse, Tx, Uint64, GENESIS_SENDER,
    },
    grug_vm_wasm::WasmVm,
    std::{
        collections::{BTreeMap, BTreeSet},
        fs, io, vec,
    },
};

const CHAIN_ID: &str = "grug-1";
const ARTIFACTS_DIR: &str = "../../../artifacts";

fn read_wasm_file(filename: &str) -> io::Result<Vec<u8>> {
    fs::read(&format!("{ARTIFACTS_DIR}/{filename}"))
}

struct TestSuite {
    app: App<MemDb, WasmVm>,
    last_finalized_block: BlockInfo,
}

impl TestSuite {
    pub fn new() -> Self {
        Self {
            app: App::new(MemDb::new()),
            last_finalized_block: BlockInfo {
                height: Uint64::default(),
                timestamp: current_time(),
                hash: Hash::ZERO,
            },
        }
    }

    pub fn init_chain(&mut self, genesis_state: GenesisState) -> AppResult<Hash> {
        self.app.do_init_chain(
            CHAIN_ID.to_string(),
            self.last_finalized_block.clone(),
            genesis_state,
        )
    }

    pub fn query(&self, req: QueryRequest) -> AppResult<QueryResponse> {
        self.app
            .do_query_app(req, self.last_finalized_block.height.into(), false)
    }

    #[allow(clippy::type_complexity)]
    pub fn _execute(
        &mut self,
        sender: &Addr,
        msgs: Vec<Message>,
    ) -> AppResult<(Hash, Vec<Event>, Vec<AppResult<Vec<Event>>>)> {
        let tx = Tx {
            sender: sender.clone(),
            msgs,
            credential: Binary::empty(),
        };

        self.last_finalized_block.height += Uint64::ONE;

        self.app
            .do_finalize_block(self.last_finalized_block.clone(), vec![(Hash::ZERO, tx)])
    }
}

#[test]
fn test() -> anyhow::Result<()> {
    let mut suite = TestSuite::new();

    let sender = Addr::mock(2);

    let initial_balances = BTreeMap::from([(
        sender.clone(),
        Coins::try_from(vec![Coin::new("ugrug", 100_u128)]).unwrap(),
    )]);

    let bank_code = to_borsh_vec(&read_wasm_file("grug_bank-aarch64.wasm").unwrap()).unwrap();
    let bank_code_hash = Hash::from_slice(sha2_256(&bank_code));
    let bank = Addr::compute(&GENESIS_SENDER, &bank_code_hash, &b"bank".to_vec().into());

    let genesis_state = GenesisState {
        config: Config {
            owner: None,
            bank,
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
                msg: to_json_value(&InstantiateMsg { initial_balances }).unwrap(),
                salt: b"bank".to_vec().into(),
                funds: Coins::new_empty(),
                admin: None,
            },
        ],
    };

    suite.init_chain(genesis_state)?;

    let res = suite.query(QueryRequest::Balance {
        address: sender.clone(),
        denom: "ugrug".to_string(),
    })?;

    // THIS IS NOT PASSING
    assert_eq!(res, QueryResponse::Balance(Coin::new("ugrug", 100_u128)));

    // Send to mock_addr 3
    // let receiver = Addr::mock(3);

    // suite.execute(&sender, vec![Message::Transfer {
    //     to: receiver.clone(),
    //     coins: vec![Coin::new("ugrug", 50_u128)].try_into().unwrap(),
    // }])
    // .unwrap();

    // let res = suite.query(QueryRequest::Balance {
    //     address: receiver.clone(),
    //     denom: "ugurg".to_string(),
    // });

    // assert_eq!(res, QueryResponse::Balance(Coin::new("ugrug", 50_u128)));

    Ok(())
}
