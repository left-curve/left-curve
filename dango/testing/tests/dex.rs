//! Test cases from:
//! <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>

use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{DANGO_DENOM, USDC_DENOM},
        dex::{self, Direction, OrderId, QueryOrdersRequest},
    },
    grug::{
        btree_map, Addressable, BalanceChange, Coins, Denom, Inner, Message, MultiplyFraction,
        NonEmpty, QuerierExt, Signer, StdResult, Udec128, Uint128,
    },
    std::collections::BTreeMap,
    test_case::test_case,
};

// --------------------------------- example 1 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // order_id = !0
        (Direction::Bid, 20, 10), // !1
        (Direction::Bid, 10, 10), // !2
        (Direction::Ask, 10, 10), // 3
        (Direction::Ask, 20, 10), // 4
        (Direction::Ask, 30, 10), // 5
    ],
    btree_map! {
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(200),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(200),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 1"
)]
// --------------------------------- example 2 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // !0
        (Direction::Bid, 20, 10), // !1
        (Direction::Bid, 10, 10), // !2
        (Direction::Ask,  5, 10), //  3
        (Direction::Ask, 15, 10), //  4
        (Direction::Ask, 25, 10), //  5
    ],
    btree_map! {
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 2"
)]
// --------------------------------- example 3 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - 50% filled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - unfilled
        (Direction::Bid, 30,  5), // !6 - filled
    ],
    btree_map! {
        !1 =>  5,
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(5),   // half filled
            USDC_DENOM.clone()  => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
        !6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(5),
            USDC_DENOM.clone()  => BalanceChange::Decreased(88), // -150 deposit, +62 refund
        },
    };
    "example 3"
)]
// --------------------------------- example 4 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 20), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - unfilled
    ],
    btree_map! {
        !1 => 10,
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(20),
            USDC_DENOM.clone()  => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(225),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(225),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 4"
)]
// --------------------------------- example 5 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 25), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - 50% filled
    ],
    btree_map! {
        !1 => 10,
        !2 => 10,
         5 =>  5,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(25),
            USDC_DENOM.clone()  => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(275),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(275),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(137), // refund: floor(5 * 27.5) = 137
        },
    };
    "example 5"
)]
fn dex_works(
    // A list of orders to submit: direction, price, amount.
    orders_to_submit: Vec<(Direction, u128, u128)>,
    // Orders that should remain not fully filled: order_id => remaining amount.
    remaining_orders: BTreeMap<OrderId, u128>,
    // Changes that should happen to the users' balances: order_id => denom => change.
    balance_changes: BTreeMap<OrderId, BTreeMap<Denom, BalanceChange>>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Find which accounts will submit the orders, so we can track their balances.
    let users_by_order_id = orders_to_submit
        .iter()
        .zip(accounts.users())
        .enumerate()
        .map(|(order_id, ((direction, ..), signer))| {
            let order_id = order_id as OrderId;
            match direction {
                Direction::Bid => (!order_id, signer.address()),
                Direction::Ask => (order_id, signer.address()),
            }
        })
        .collect::<BTreeMap<_, _>>();

    // Track the users' balances.
    suite
        .balances()
        .record_many(users_by_order_id.values().copied());

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128::new(price);
            let amount = Uint128::new(amount);

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(USDC_DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(DANGO_DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::SubmitOrder {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction,
                    amount,
                    price,
                },
                funds,
            )?;

            signer.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    suite.make_block(txs);

    // Check the users' balances should have changed correctly.
    for (order_id, changes) in balance_changes {
        suite
            .balances()
            .should_change(users_by_order_id[&order_id], changes);
    }

    // Check the remaining unfilled orders.
    let orders = suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .unwrap()
        .into_iter()
        .map(|(order_id, order)| (order_id, order.remaining.into_inner()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(orders, remaining_orders);
}
