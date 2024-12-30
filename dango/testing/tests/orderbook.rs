//! Test cases from:
//! <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>

use {
    dango_testing::{setup_test_naive, TestSuite},
    dango_types::orderbook::{self, Direction, OrderId, QueryOrdersRequest},
    grug::{
        btree_map, Addr, Addressable, Coins, Denom, Inner, Message, MultiplyFraction, NonEmpty,
        Signer, StdResult, Udec128, Uint128,
    },
    grug_app::NaiveProposalPreparer,
    std::{collections::BTreeMap, str::FromStr, sync::LazyLock},
    test_case::test_case,
};

static BASE_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("udng").unwrap());
static QUOTE_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

enum BalanceChange {
    Increased(u128),
    Decreased(u128),
    Unchanged,
}

// TODO: this can be included in `TestSuite`.
#[derive(Default)]
struct BalanceTracker {
    old_balances: BTreeMap<Addr, Coins>,
}

impl BalanceTracker {
    pub fn record_balances<I>(&mut self, suite: &TestSuite<NaiveProposalPreparer>, accounts: I)
    where
        I: IntoIterator<Item = Addr>,
    {
        self.old_balances = accounts
            .into_iter()
            .map(|addr| (addr, suite.query_balances(&addr).unwrap()))
            .collect();
    }

    pub fn assert(
        &self,
        suite: &TestSuite<NaiveProposalPreparer>,
        account: Addr,
        changes: BTreeMap<Denom, BalanceChange>,
    ) {
        let old_balances = self.old_balances.get(&account).unwrap();
        let new_balances = suite.query_balances(&account).unwrap();

        for (denom, change) in changes {
            let old_balance = old_balances.amount_of(&denom);
            let new_balance = new_balances.amount_of(&denom);
            match change {
                BalanceChange::Increased(diff) => {
                    assert_eq!(
                        new_balance,
                        old_balance + Uint128::new(diff),
                        "incorrect balance! account: {}, denom: {}, amount: {} != {} + {}",
                        account,
                        denom,
                        new_balance,
                        old_balance,
                        diff
                    );
                },
                BalanceChange::Decreased(diff) => {
                    assert_eq!(
                        new_balance,
                        old_balance - Uint128::new(diff),
                        "incorrect balance! account: {}, denom: {}, amount: {} != {} - {}",
                        account,
                        denom,
                        new_balance,
                        old_balance,
                        diff
                    );
                },
                BalanceChange::Unchanged => {
                    assert_eq!(
                        new_balance, old_balance,
                        "incorrect balance! account: {}, denom: {}, amount: {} != {}",
                        account, denom, new_balance, old_balance
                    );
                },
            }
        }
    }
}

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
            BASE_DENOM.clone()  => BalanceChange::Increased(10),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(200),
        },
        !1 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Increased(10),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(200),
        },
        4 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(200),
        },
        5 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Unchanged,
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
            BASE_DENOM.clone()  => BalanceChange::Increased(10),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Increased(10),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(175),
        },
        5 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Unchanged,
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
            BASE_DENOM.clone()  => BalanceChange::Increased(10),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Increased(5),   // half filled
            QUOTE_DENOM.clone() => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        !2 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(175),
        },
        5 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Unchanged,
        },
        !6 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Increased(5),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(88), // -150 deposit, +62 refund
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
            BASE_DENOM.clone()  => BalanceChange::Increased(20),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        !1 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(225),
        },
        4 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(225),
        },
        5 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Unchanged,
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
            BASE_DENOM.clone()  => BalanceChange::Increased(25),
            QUOTE_DENOM.clone() => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        !1 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Unchanged,
            QUOTE_DENOM.clone() => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(275),
        },
        4 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(275),
        },
        5 => btree_map! {
            BASE_DENOM.clone()  => BalanceChange::Decreased(10),
            QUOTE_DENOM.clone() => BalanceChange::Increased(137), // refund: floor(5 * 27.5) = 137
        },
    };
    "example 5"
)]
fn orderbook_works(
    // A list of orders to submit: direction, price, amount.
    orders_to_submit: Vec<(Direction, u128, u128)>,
    // Orders that should remain unfilled: order_id => remaining amount.
    remaining_orders: BTreeMap<OrderId, u128>,
    // Changes that should happen to the traders' balances: order_id => denom => change.
    balance_changes: BTreeMap<OrderId, BTreeMap<Denom, BalanceChange>>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();
    let mut tracker = BalanceTracker::default();

    // Find which accounts will submit the orders, so we can track their balances.
    let traders_by_order_id = orders_to_submit
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

    // Track the traders' balances.
    tracker.record_balances(&suite, traders_by_order_id.values().copied());

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
                    Coins::one(QUOTE_DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(BASE_DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.orderbook,
                &orderbook::ExecuteMsg::SubmitOrder {
                    base_denom: BASE_DENOM.clone(),
                    quote_denom: QUOTE_DENOM.clone(),
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

    // Check the traders' balances should have changed correctly.
    for (order_id, changes) in balance_changes {
        tracker.assert(&suite, traders_by_order_id[&order_id], changes);
    }

    // Check the remaining unfilled orders.
    let orders = suite
        .query_wasm_smart(contracts.orderbook, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .unwrap()
        .into_iter()
        .map(|(order_id, order)| (order_id, order.remaining.into_inner()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(orders, remaining_orders);
}
