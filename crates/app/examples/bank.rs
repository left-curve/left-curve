//! How to run this example:
//!
//! $ just optimize
//! $ cargo run -p cw-app --example bank

use {
    cfg_if::cfg_if,
    cw_app::App,
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_std::{
        from_json, hash, to_json, Addr, Binary, BlockInfo, Coin, GenesisState, Message,
        MockStorage, Query, Storage, Tx, Uint128,
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
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    println!("🤖 Creating app");
    let mut app = App::new(MockStorage::new());

    println!("🤖 Reading wasm byte code from file");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let wasm_file_path = manifest_dir.join({
        cfg_if! {
            if #[cfg(target_arch = "aarch64")] {
                "../../artifacts/cw_bank-aarch64.wasm"
            } else {
                "../../artifacts/cw_bank.wasm"
            }
        }
    });
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
            wasm_byte_code: wasm_byte_code.clone().into(),
        },
        Message::Instantiate {
            code_hash: code_hash.clone(),
            msg: to_json(&InstantiateMsg {
                initial_balances: initial_balances(),
            })?,
            salt:  salt.clone(),
            funds: vec![],
            admin: None,
        },
    ])];
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("🤖 Making transfers");
    let block = mock_block_info(2, 2);
    let mut txs = vec![mock_tx(1, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(4, "uatom", 75))?,
            funds:    vec![],
        },
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(5, "uosmo", 420))?,
            funds:    vec![],
        },
    ])];
    txs.push(mock_tx(2, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(3, "uatom", 50))?,
            funds:    vec![],
        },
    ]));
    txs.push(mock_tx(3, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(1, "uatom", 69))?,
            funds:    vec![],
        },
    ]));
    txs.push(mock_tx(5, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(6, "uosmo", 64))?,
            funds:    vec![],
        },
    ]));
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("😈 Intentionally making some failed txs");
    let block = mock_block_info(3, 3);
    // uploading the same code twice - fail
    let mut txs = vec![mock_tx(0, vec![
        Message::StoreCode {
            wasm_byte_code: wasm_byte_code.into(),
        },
    ])];
    // instantiate a contract with an non-existent code hash - fail
    txs.push(mock_tx(0, vec![
        Message::Instantiate {
            code_hash: hash("haha"),
            msg: to_json(&InstantiateMsg {
                initial_balances: vec![],
            })?,
            salt:  salt.clone(),
            funds: vec![],
            admin: None,
        },
    ]));
    // instantiate a contract that would have the same address - fail
    txs.push(mock_tx(0, vec![
        Message::Instantiate {
            code_hash,
            msg: to_json(&InstantiateMsg {
                initial_balances: vec![],
            })?,
            salt,
            funds: vec![],
            admin: None,
        },
    ]));
    // execute a non-existent contract - fail
    txs.push(mock_tx(0, vec![
        Message::Execute {
            contract: Addr::mock(123),
            msg:      to_json(&send_msg(4, "uatom", 75))?,
            funds:    vec![],
        },
    ]));
    // send more coins than balance - fail
    txs.push(mock_tx(1, vec![
        Message::Execute {
            contract: contract_addr.clone(),
            msg:      to_json(&send_msg(4, "uatom", 999999999))?,
            funds:    vec![],
        },
    ]));
    app.finalize_block(block, txs)?;
    app.commit()?;

    println!("🤖 Querying chain info");
    query(&mut app, Query::Info {})?;

    println!("🤖 Querying codes");
    query(&mut app, Query::Codes {
        start_after: None,
        limit:       None,
    })?;

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
        chain_id: "dev-1".into(),
        height,
        timestamp,
    }
}

fn mock_tx(sender_idx: u8, msgs: Vec<Message>) -> Tx {
    Tx {
        sender:     Addr::mock(sender_idx),
        credential: Binary::empty(),
        msgs,
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

fn send_msg(to_idx: u8, denom: &str, amount: u128) -> ExecuteMsg {
    ExecuteMsg::Send {
        to:     Addr::mock(to_idx),
        denom:  denom.into(),
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
