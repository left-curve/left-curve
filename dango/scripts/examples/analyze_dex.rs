use {
    dango_genesis::GenesisCodes,
    dango_types::{DangoQuerier, dex},
    grug::{
        BlockInfo, Coin, Coins, DecCoin, DecCoins, Duration, Hash256, Number, QuerierExt,
        TestSuite, Timestamp,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_disk_lite::DiskDbLite,
    grug_vm_rust::RustVm,
    std::u32,
};

const DB_PATH: &str = "";

fn main() -> Result<(), anyhow::Error> {
    // startup
    let app = {
        RustVm::genesis_codes();

        TestSuite::new_with_app(
            App::new(
                DiskDbLite::open(DB_PATH, None::<&(&[u8], &[u8])>)?,
                RustVm::new(),
                NaiveProposalPreparer,
                NullIndexer,
                100000,
                None,
            ),
            "foo".to_string(),
            BlockInfo {
                height: 1,
                timestamp: Timestamp::default(),
                hash: Hash256::ZERO,
            },
            Duration::default(),
            1,
        )
    };

    let cfg = app.query_dango_config()?;

    let dex = cfg.addresses.dex;

    let mut balance = app.query_balances(&dex)?;

    print_coins("current dex balance", &balance);

    let orders = {
        let orders = app.query_wasm_smart(dex, dex::QueryOrdersRequest {
            start_after: None,
            limit: Some(u32::MAX),
        })?;

        println!("\norders founds: {}", orders.len());

        orders
            .values()
            .try_fold(DecCoins::new(), |mut coins, order| {
                if order.user != dex {
                    match order.direction {
                        dex::Direction::Bid => {
                            let remaining_in_quote = order.remaining.checked_mul(order.price)?;

                            coins.insert(DecCoin {
                                denom: order.quote_denom.clone(),
                                amount: remaining_in_quote,
                            })?;
                        },
                        dex::Direction::Ask => {
                            coins.insert(DecCoin {
                                denom: order.base_denom.clone(),
                                amount: order.remaining,
                            })?;
                        },
                    }
                }

                Ok::<_, anyhow::Error>(coins)
            })?
            .into_coins_floor()
    };

    print_coins("orders balance", &orders);

    balance.deduct_many(orders)?;

    print_coins("\nbalance without orders", &balance);

    let reserve = {
        let reserves = app.query_wasm_smart(dex, dex::QueryReservesRequest {
            start_after: None,
            limit: Some(u32::MAX),
        })?;

        reserves
            .into_iter()
            .try_fold(Coins::new(), |mut coins, reserve| {
                for i in [reserve.reserve.first(), reserve.reserve.second()] {
                    let coin = Coin::new(i.denom.clone(), *i.amount)?;
                    coins.insert(coin)?;
                }

                Ok::<_, anyhow::Error>(coins)
            })?
    };

    print_coins("\nreserve", &reserve);

    balance.deduct_many(reserve)?;

    print_coins("\nbalance without orders and reserve", &balance);

    Ok(())
}

fn print_coins(source: &'static str, coins: &Coins) {
    println!("{source}");
    for coin in coins {
        println!("{}: {}", coin.denom, coin.amount)
    }
}
