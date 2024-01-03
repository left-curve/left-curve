use {
    cw_app::App,
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_std::{
        from_json, hash, to_json, Addr, BlockInfo, GenesisState, Message, MockStorage, Query, Tx,
        Uint128, WasmSmartResponse,
    },
    std::{env, fs::File, io::Read, path::PathBuf},
    tracing::info,
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

    info!("creating app");
    let mut app = App::new(MockStorage::new());

    info!("reading wasm byte code from file");
    let wasm_file_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../../target/wasm32-unknown-unknown/debug/cw_bank.wasm");
    let mut wasm_file = File::open(wasm_file_path)?;
    let mut wasm_byte_code = Vec::new();
    wasm_file.read_to_end(&mut wasm_byte_code)?;

    info!("computing bank contract address");
    let code_hash = hash(&wasm_byte_code);
    let salt = b"cw-bank".to_vec().into();
    let contract_addr = Addr::compute(&code_hash, &salt);

    info!("initialize chain");
    app.init_chain(GenesisState {
        chain_id: "dev-1".into(),
        msgs:     vec![],
    })?;

    info!("uploading code and instantiating contract");
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

    info!("making transfers");
    let block = mock_block_info(2, 2);
    let txs = vec![
        mock_tx(1, vec![
            send_msg(&contract_addr, 1, 4, "uatom", 75)?,
            send_msg(&contract_addr, 1, 5, "uosmo", 420)?,
        ]),
        mock_tx(2, vec![send_msg(&contract_addr, 2, 3, "uatom", 50)?]),
        mock_tx(3, vec![send_msg(&contract_addr, 3, 1, "uatom", 69)?]),
        mock_tx(5, vec![send_msg(&contract_addr, 5, 6, "uosmo", 64)?]),
    ];
    app.finalize_block(block, txs)?;
    app.commit()?;

    info!("querying chain info");
    let res_bytes = app.query(Query::Info {})?;
    let res_str = String::from_utf8(res_bytes.as_ref().to_vec())?;
    println!("{res_str}");

    info!("querying accounts");
    let res_bytes = app.query(Query::Accounts {
        start_after: None,
        limit: None,
    })?;
    let res_str = String::from_utf8(res_bytes.as_ref().to_vec())?;
    println!("{res_str}");

    info!("querying balances");
    let res_bytes = app.query(Query::WasmSmart {
        contract: contract_addr,
        msg: to_json(&QueryMsg::Balances {
            start_after: None,
            limit: None,
        })?,
    })?;
    let res: WasmSmartResponse = from_json(&res_bytes)?;
    let res_str = String::from_utf8(res.data.as_ref().to_vec())?;
    println!("{res_str}");

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

fn send_msg(
    contract: &Addr,
    from_idx: u8,
    to_idx:   u8,
    denom:    &str,
    amount:   u128,
) -> anyhow::Result<Message> {
    Ok(Message::Execute {
        contract: contract.clone(),
        msg: to_json(&ExecuteMsg::Send {
            from:   Addr::mock(from_idx),
            to:     Addr::mock(to_idx),
            denom:  denom.into(),
            amount: Uint128::new(amount),
        })?,
        funds: vec![],
    })
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
