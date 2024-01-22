//! How to run this example:
//!
//! $ just optimize
//! $ cargo run -p cw-vm --example bank

use {
    cfg_if::cfg_if,
    cw_bank::{Balance, InstantiateMsg},
    cw_db::{BackendStorage, MockBackendStorage},
    cw_std::{to_json, Addr, BankQuery, BlockInfo, Coin, Coins, Context, TransferMsg, Uint128},
    cw_vm::{BackendQuerier, Instance, MockBackendQuerier},
    lazy_static::lazy_static,
    std::{env, fs::File, io::Read, path::PathBuf},
};

lazy_static! {
    static ref INITIAL_BALANCES: Vec<Balance> = vec![
        Balance {
            address: Addr::mock(1),
            coins: Coins::from_vec_unchecked(vec![
                Coin {
                    denom: "uatom".into(),
                    amount: Uint128::new(100),
                },
                Coin {
                    denom: "uosmo".into(),
                    amount: Uint128::new(888),
                },
            ]),
        },
        Balance {
            address: Addr::mock(2),
            coins: Coins::from(Coin {
                denom: "uatom".into(),
                amount: Uint128::new(50),
            }),
        },
        Balance {
            address: Addr::mock(3),
            coins: Coins::from(Coin {
                denom: "uatom".into(),
                amount: Uint128::new(123),
            }),
        },
    ];
}

fn main() -> anyhow::Result<()> {
    // set tracing to TRACE level, so that we can see DB reads/writes logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    // create Wasm host instance
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
    let mut wasm_byte_code = vec![];
    wasm_file.read_to_end(&mut wasm_byte_code)?;

    let store = MockBackendStorage::new();
    let mut instance = Instance::build_from_code(store, MockBackendQuerier, &wasm_byte_code)?;

    // deploy the contract
    instantiate(&mut instance)?;

    // make some transfers
    send(&mut instance, Addr::mock(1), Addr::mock(4), "uatom", 75)?;
    send(&mut instance, Addr::mock(1), Addr::mock(5), "uosmo", 420)?;
    send(&mut instance, Addr::mock(2), Addr::mock(3), "uatom", 50)?;
    send(&mut instance, Addr::mock(3), Addr::mock(1), "uatom", 69)?;
    send(&mut instance, Addr::mock(5), Addr::mock(6), "uosmo", 64)?;

    // query and print out the balances
    // should be:
    // 0x1: 94 uatom, 468 uosmo
    // 0x2: no balance, deleted from storage
    // 0x3: 104 uatom
    // 0x4: 75 uatom
    // 0x5: 356 uosmo
    // 0x6: 64 uosmo
    query_balances(&mut instance)?;

    println!("✅ Done!");

    Ok(())
}

fn instantiate<S, Q>(instance: &mut Instance<S, Q>) -> anyhow::Result<()>
where
    S: BackendStorage + 'static,
    Q: BackendQuerier + 'static,
{
    println!("🤖 Instantiating contract");

    let res = instance.call_instantiate(
        &mock_context(Some(Addr::mock(0))),
        to_json(&InstantiateMsg {
            initial_balances: INITIAL_BALANCES.clone(),
        })?,
    )?;

    println!("✅ Instantiation successful! res={}", serde_json::to_string(&res)?);

    Ok(())
}

fn send<S, Q>(
    instance: &mut Instance<S, Q>,
    from:     Addr,
    to:       Addr,
    denom:    &str,
    amount:   u128,
) -> anyhow::Result<()>
where
    S: BackendStorage + 'static,
    Q: BackendQuerier + 'static,
{
    println!("🤖 Sending... from={from:?} to={to:?} denom={denom} amount={amount}");

    let res = instance.call_transfer(
        &mock_context(None),
        &TransferMsg {
            from,
            to,
            coins: Coins::from(Coin {
                denom:  denom.into(),
                amount: Uint128::new(amount),
            }),
        },
    )?;

    println!("✅ Send successful! res={}", serde_json::to_string(&res)?);

    Ok(())
}

fn query_balances<S, Q>(instance: &mut Instance<S, Q>) -> anyhow::Result<()>
where
    S: BackendStorage + 'static,
    Q: BackendQuerier + 'static,
{
    println!("🤖 Querying balances");

    let mut balances = vec![];
    for idx in 1..=6 {
        let address = Addr::mock(idx);

        let coins = instance
            .call_query_bank(
                &mock_context(None),
                &BankQuery::Balances {
                    address:     address.clone(),
                    start_after: None,
                    limit:       None,
                }
            )?
            .into_std_result()?
            .as_balances();

        balances.push(Balance { address, coins });
    }

    println!("{}", serde_json::to_string_pretty(&balances)?);

    Ok(())
}

// for this example, we don't use a context that resembles a real blockchain.
// see the example in cw-app instead.
fn mock_context(sender: Option<Addr>) -> Context {
    Context {
        block: BlockInfo {
            chain_id:  "dev-1".into(),
            height:    0,
            timestamp: 0,
        },
        contract: Addr::mock(0),
        funds:    None,
        simulate: None,
        sender,
    }
}
