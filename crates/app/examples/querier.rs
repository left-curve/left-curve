use {
    cw_app::App,
    cw_db::MockStorage,
    cw_mock_querier::QueryMsg,
    cw_std::{hash, to_json, Addr, Config, Empty, GenesisState, Message, QueryRequest, Storage},
    serde::ser::Serialize,
    std::{env, fs::File, io::Read, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    println!("🤖 Creating app");
    let mut app = App::new(MockStorage::new());

    println!("🤖 Reading wasm byte code from file");
    let artifacts_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../artifacts");
    let wasm_file_path = {
        #[cfg(target_arch = "aarch64")]
        { artifacts_dir.join("cw_account-aarch64.wasm") }
        #[cfg(not(target_arch = "aarch64"))]
        { artifacts_dir.join("cw_account.wasm") }
    };
    let mut wasm_file = File::open(wasm_file_path)?;
    let mut wasm_byte_code = Vec::new();
    wasm_file.read_to_end(&mut wasm_byte_code)?;

    println!("🤖 Computing querier contract address");
    let code_hash = hash(&wasm_byte_code);
    let salt = b"mock-querier".to_vec().into();
    let address = Addr::compute(&Addr::mock(0), &code_hash, &salt);

    println!("🤖 Genesis chain, instantiate querier contract");
    app.init_chain(GenesisState {
        chain_id: "dev-1".to_string(),
        config: Config {
            // we don't need a bank contract for this demo
            bank: Addr::mock(0),
        },
        msgs: vec![
            Message::StoreCode {
                wasm_byte_code: wasm_byte_code.into(),
            },
            Message::Instantiate {
                code_hash,
                msg: to_json(&Empty {})?,
                salt,
                funds: vec![],
                admin: None,
            },
        ],
    })?;

    println!("🤖 Querying chain info...");
    query_wasm_smart(&mut app, &address, &QueryMsg::QueryChain {
        request: QueryRequest::Info {},
    })?;

    Ok(())
}

fn query_wasm_smart<S, M>(app: &mut App<S>, contract: &Addr, msg: &M) -> anyhow::Result<()>
where
    S: Storage + 'static,
    M: Serialize,
{
    let resp = app.query(QueryRequest::WasmSmart {
        contract: contract.clone(),
        msg: to_json(msg)?,
    })?;
    println!("{}", resp.as_wasm_smart().data);
    Ok(())
}
