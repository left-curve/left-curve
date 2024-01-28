use {
    cfg_if::cfg_if,
    cw_app::App,
    cw_db::MockStorage,
    cw_mock_querier::QueryMsg,
    cw_std::{
        from_json, hash, to_json, Addr, BlockInfo, Coins, Config, Empty, GenesisState, Message,
        QueryRequest, QueryResponse, Storage,
    },
    serde::ser::Serialize,
    std::{env, fs::File, io::Read, path::PathBuf},
};

const MOCK_CHAIN_ID: &str = "dev-1";

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    println!("🤖 Creating app");
    let app = App::new(MockStorage::new());

    println!("🤖 Reading wasm byte code from file");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let wasm_file_path = manifest_dir.join({
        cfg_if! {
            if #[cfg(target_arch = "aarch64")] {
                "../../artifacts/cw_mock_querier-aarch64.wasm"
            } else {
                "../../artifacts/cw_mock_querier.wasm"
            }
        }
    });
    let mut wasm_file = File::open(wasm_file_path)?;
    let mut wasm_byte_code = Vec::new();
    wasm_file.read_to_end(&mut wasm_byte_code)?;

    println!("🤖 Computing querier contract address");
    let code_hash = hash(&wasm_byte_code);
    let salt = b"mock-querier".to_vec().into();
    let address = Addr::compute(&Addr::mock(0), &code_hash, &salt);

    println!("🤖 Genesis chain, instantiate querier contract");
    app.do_init_chain(MOCK_CHAIN_ID.into(), mock_block_info(0, 0), &to_json(&GenesisState {
        config: Config {
            // we don't need an owner or a bank contract for this demo
            owner: None,
            bank:  Addr::mock(0),
        },
        msgs: vec![
            Message::StoreCode {
                wasm_byte_code: wasm_byte_code.into(),
            },
            Message::Instantiate {
                code_hash,
                msg: to_json(&Empty {})?,
                salt,
                funds: Coins::new_empty(),
                admin: None,
            },
        ],
    })?)?;

    println!("🤖 Querying chain info...");
    query_wasm_smart(&app, &address, &QueryMsg::QueryChain {
        request: QueryRequest::Info {},
    })?;

    println!("🤖 Querying codes...");
    query_wasm_smart(&app, &address, &QueryMsg::QueryChain {
        request: QueryRequest::Codes {
            start_after: None,
            limit:       None,
        },
    })?;

    println!("🤖 Querying accounts...");
    query_wasm_smart(&app, &address, &QueryMsg::QueryChain {
        request: QueryRequest::Accounts {
            start_after: None,
            limit:       None,
        },
    })?;

    Ok(())
}

fn mock_block_info(height: u64, timestamp: u64) -> BlockInfo {
    BlockInfo {
        height,
        timestamp,
    }
}

fn query_wasm_smart<S, M>(app: &App<S>, contract: &Addr, msg: &M) -> anyhow::Result<()>
where
    S: Storage + 'static,
    M: Serialize,
{
    let data = from_json::<QueryResponse>(app.do_query(&to_json(&QueryRequest::WasmSmart {
        contract: contract.clone(),
        msg: to_json(msg)?,
    })?)?)?
    .as_wasm_smart()
    .data;

    println!("{}", data);

    Ok(())
}
