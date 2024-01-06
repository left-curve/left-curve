//! How to run this example:
//!
//! $ just optimize
//! $ cargo run -p cw-app --example bank

use {
    cw_app::App,
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_std::{
        from_json, hash, to_json, Addr, BlockInfo, Coin, GenesisState, Message, MockStorage, Query,
        Storage, Tx, Uint128,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{env, fs::File, io::Read, path::PathBuf},
};

// (address, denom, amount)
const INITIAL_BALANCES: [(Addr, &str, Uint128); 4] = [
    (Addr::mock(1), "uatom", Uint128::new(100)),
    (Addr::mock(1), "uosmo", Uint128::new(888)),
    (Addr::mock(2), "uatom", Uint128::new(50)),
    (Addr::mock(3), "uatom", Uint128::new(123)),
];

fn main() -> anyhow::Result<()> {
    // set tracing to TRACE level, so that we can see DB reads/writes logs
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();

    println!("🤖 Creating app");
    let mut app = App::new(MockStorage::new());

    println!("🤖 Reading wasm byte code from file");
    let artifacts_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../artifacts");
    let wasm_file_path = {
        #[cfg(target_arch = "aarch64")]
        { artifacts_dir.join("cw_bank-aarch64.wasm") }
        #[cfg(not(target_arch = "aarch64"))]
        { artifacts_dir.join("cw_bank.wasm") }
    };
    let mut wasm_file = File::open(wasm_file_path)?;
    let mut wasm_byte_code = Vec::new();
    wasm_file.read_to_end(&mut wasm_byte_code)?;

    println!("🤖 Computing bank contract address");
    let code_hash = hash(&wasm_byte_code);
    let salt = b"cw-bank".to_vec().into();
    let contract_addr = Addr::compute(&code_hash, &salt);

    println!("🤖 Initialize chain");
    app.init_chain(GenesisState {
        chain_id: "dev-1".into(),
        msgs:     vec![],
    })?;

    println!("🤖 Uploading code and instantiating contract");
    let block = mock_block_info(1, 1);
    let txs = vec![mock_tx(0, vec![
        Message::StoreCode {
            wasm_byte_code: wasm_byte_code.into(),
        },
        Message::Instantiate {
            code_hash,
            msg: to_json(&InstantiateMsg {
                initial_balances: initial_balances(),
            })?,
            salt,
            funds: vec![],
            admin: None,
        }
    ])];
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("🤖 Making transfers");
    let block = mock_block_info(2, 2);
    let mut txs = vec![];
    txs.push(mock_tx(1, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(1, 4, "uatom", 75))?,
            funds:    vec![],
        },
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(1, 5, "uosmo", 420))?,
            funds:    vec![],
        },
    ]));
    txs.push(mock_tx(2, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(2, 3, "uatom", 50))?,
            funds:    vec![],
        },
    ]));
    txs.push(mock_tx(3, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(3, 1, "uatom", 69))?,
            funds:    vec![],
        },
    ]));
    txs.push(mock_tx(2, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(5, 6, "uosmo", 64))?,
            funds:    vec![],
        },
    ]));
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("🤖 Querying chain info");
    query(&mut app, Query::Info {})?;

    println!("🤖 Querying accounts");
    query(&mut app, Query::Accounts {
        start_after: None,
        limit:       None,
    })?;

    println!("🤖 Querying balances");
    query_wasm_smart::<_, _, Vec<Balance>>(&mut app, &contract_addr, &QueryMsg::Balances {
        start_after: None,
        limit:       None,
    })?;

    println!("🤖 Querying balances of a specific user (0x1)");
    query_wasm_smart::<_, _, Vec<Coin>>(&mut app, &contract_addr, &QueryMsg::BalancesByUser {
        address:     Addr::mock(1),
        start_after: None,
        limit:       None,
    })?;

    println!("✅ Done!");

    Ok(())
}

fn mock_block_info(height: u64, timestamp: u64) -> BlockInfo {
    BlockInfo {
        height,
        timestamp,
    }
}

fn mock_tx(sender_idx: u8, msgs: Vec<Message>) -> Tx {
    Tx {
        sender: Addr::mock(sender_idx),
        msgs,
        credential: None,
    }
}

fn initial_balances() -> Vec<Balance> {
    let mut balances = vec![];
    for (address, denom, amount) in INITIAL_BALANCES {
        balances.push(Balance {
            address,
            denom: denom.into(),
            amount,
        });
    }
    balances
}

fn send_msg(from_idx: u8, to_idx: u8, denom: &str, amount: u128) -> ExecuteMsg {
    ExecuteMsg::Send {
        from: Addr::mock(from_idx),
        to: Addr::mock(to_idx),
        denom: denom.into(),
        amount: Uint128::new(amount),
    }
}

fn query<S>(app: &mut App<S>, req: Query) -> anyhow::Result<()>
where
    S: Storage + 'static,
{
    let resp = app.query(req)?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

fn query_wasm_smart<S, M, T>(app: &mut App<S>, contract: &Addr, msg: &M) -> anyhow::Result<()>
where
    S: Storage + 'static,
    M: Serialize,
    T: Serialize + DeserializeOwned,
{
    let resp = app.query(Query::WasmSmart {
        contract: contract.clone(),
        msg: to_json(msg)?,
    })?;
    let resp: T = from_json(resp.as_wasm_smart().data)?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
