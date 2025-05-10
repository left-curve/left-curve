use {
    dango_testing::setup_test_naive,
    dango_types::{
        bank::{
            OrphanedTransferResponseItem, QueryOrphanedTransfersByRecipientRequest,
            QueryOrphanedTransfersBySenderRequest, QueryOrphanedTransfersRequest, Received, Sent,
            TransferOrphaned,
        },
        constants::{dango, usdc},
    },
    grug::{
        Addressable, BalanceChange, CheckedContractEvent, JsonDeExt, QuerierExt, ResultExt,
        SearchEvent, addr, btree_map, coins,
    },
};

#[test]
fn batch_transfer() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Create two non-existent recipient addresses.
    let dead1 = addr!("000000000000000000000000000000000000dead");
    let dead2 = addr!("00000000000000000000000000000000deaddead");

    suite.balances().record_many([
        &accounts.owner.address(),
        &accounts.user1.address(),
        &accounts.user2.address(),
        &contracts.bank,
        &dead1,
        &dead2,
    ]);

    // Owner makes a multi-send to users 1, 2, and the non-existent recipient.
    let events = suite
        .batch_transfer(&mut accounts.owner, btree_map! {
            accounts.user1.address() => coins! {
                dango::DENOM.clone() => 100,
            },
            accounts.user2.address() => coins! {
                dango::DENOM.clone() => 200,
                usdc::DENOM.clone() => 300,
            },
            dead1 => coins! {
                dango::DENOM.clone() => 400,
            },
            dead2 => coins! {
                usdc::DENOM.clone() => 500,
            },
        })
        .should_succeed()
        .events;

    // Check the emitted events are correct.
    // `sent` events:
    {
        let sends = events
            .clone()
            .search_event::<CheckedContractEvent>()
            .with_predicate(|e| e.ty == "sent")
            .take()
            .all()
            .into_iter()
            .map(|e| e.event.data.deserialize_json::<Sent>().unwrap())
            .collect::<Vec<_>>();

        assert!(vectors_have_same_elements(sends, vec![
            Sent {
                user: accounts.owner.address(),
                to: accounts.user1.address(),
                coins: coins! {
                    dango::DENOM.clone() => 100,
                },
            },
            Sent {
                user: accounts.owner.address(),
                to: accounts.user2.address(),
                coins: coins! {
                    dango::DENOM.clone() => 200,
                    usdc::DENOM.clone() => 300,
                },
            },
            Sent {
                user: accounts.owner.address(),
                to: contracts.bank, // The orphaned transfer goes to the bank.
                coins: coins! {
                    dango::DENOM.clone() => 400,
                },
            },
            Sent {
                user: accounts.owner.address(),
                to: contracts.bank, // The orphaned transfer goes to the bank.
                coins: coins! {
                    usdc::DENOM.clone() => 500,
                },
            },
        ]));
    }

    // `received` events:
    {
        let receives = events
            .clone()
            .search_event::<CheckedContractEvent>()
            .with_predicate(|e| e.ty == "received")
            .take()
            .all()
            .into_iter()
            .map(|e| e.event.data.deserialize_json::<Received>().unwrap())
            .collect::<Vec<_>>();

        assert!(vectors_have_same_elements(receives, vec![
            Received {
                user: accounts.user1.address(),
                from: accounts.owner.address(),
                coins: coins! {
                    dango::DENOM.clone() => 100,
                },
            },
            Received {
                user: accounts.user2.address(),
                from: accounts.owner.address(),
                coins: coins! {
                    dango::DENOM.clone() => 200,
                    usdc::DENOM.clone() => 300,
                },
            },
            Received {
                user: contracts.bank, // The orphaned transfer goes to the bank.
                from: accounts.owner.address(),
                coins: coins! {
                    dango::DENOM.clone() => 400,
                },
            },
            Received {
                user: contracts.bank, // The orphaned transfer goes to the bank.
                from: accounts.owner.address(),
                coins: coins! {
                    usdc::DENOM.clone() => 500,
                },
            },
        ]));
    }

    // `transfer_orphaned` events:
    {
        let orphans = events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|e| e.ty == "transfer_orphaned")
            .take()
            .all()
            .into_iter()
            .map(|e| e.event.data.deserialize_json::<TransferOrphaned>().unwrap())
            .collect::<Vec<_>>();

        assert!(vectors_have_same_elements(orphans, vec![
            TransferOrphaned {
                from: accounts.owner.address(),
                to: dead1,
                coins: coins! {
                    dango::DENOM.clone() => 400,
                },
            },
            TransferOrphaned {
                from: accounts.owner.address(),
                to: dead2,
                coins: coins! {
                    usdc::DENOM.clone() => 500,
                },
            },
        ]));
    }

    // Check the balance changes.
    {
        suite.balances().should_change(&accounts.owner, btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(700),
            usdc::DENOM.clone() => BalanceChange::Decreased(800),
        });

        suite.balances().should_change(&accounts.user1, btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100),
        });

        suite.balances().should_change(&accounts.user2, btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(200),
            usdc::DENOM.clone() => BalanceChange::Increased(300),
        });

        // The token that are supposed to go to the non-existing accounts should
        // have been withheld in the bank contract as orphaned transfers.
        suite.balances().should_change(&contracts.bank, btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(400),
            usdc::DENOM.clone() => BalanceChange::Increased(500),
        });

        // The non-existing accounts should have no balance.
        suite.balances().should_change(&dead1, btree_map! {});
        suite.balances().should_change(&dead2, btree_map! {});
    }

    // The orphaned transfers should have been recorded.
    {
        suite
            .query_wasm_smart(contracts.bank, QueryOrphanedTransfersRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(vec![
                OrphanedTransferResponseItem {
                    sender: accounts.owner.address(),
                    recipient: dead1,
                    amount: coins! { dango::DENOM.clone() => 400 },
                },
                OrphanedTransferResponseItem {
                    sender: accounts.owner.address(),
                    recipient: dead2,
                    amount: coins! { usdc::DENOM.clone() => 500 },
                },
            ]);

        suite
            .query_wasm_smart(contracts.bank, QueryOrphanedTransfersBySenderRequest {
                sender: accounts.owner.address(),
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map! {
                dead1 => coins! { dango::DENOM.clone() => 400 },
                dead2 => coins! { usdc::DENOM.clone() => 500 },
            });

        suite
            .query_wasm_smart(contracts.bank, QueryOrphanedTransfersByRecipientRequest {
                recipient: dead1,
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map! {
                accounts.owner.address() => coins! { dango::DENOM.clone() => 400 },
            });

        suite
            .query_wasm_smart(contracts.bank, QueryOrphanedTransfersByRecipientRequest {
                recipient: dead2,
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(btree_map! {
                accounts.owner.address() => coins! { usdc::DENOM.clone() => 500 },
            });
    }
}

/// Returns whether two vectors have the same elements, not necessarily in the
/// same order.
///
/// This assumes the type `T` doesn't implement `Hash` or `Ord`, so we can't
/// trivially do this with a `HashSet` or `BTreeSet`.
fn vectors_have_same_elements<T>(mut a: Vec<T>, mut b: Vec<T>) -> bool
where
    T: PartialEq,
{
    // If lengths differ, they can't have the same elements.
    if a.len() != b.len() {
        return false;
    }

    // Process each element in `a`.
    while let Some(elem) = a.pop() {
        // Try to find and remove a matching element in `b`.
        if let Some(index) = b.iter().position(|e| e == &elem) {
            b.remove(index);
        } else {
            // If the element is not found in `b`, the vectors are different.
            return false;
        }
    }

    // If `b` is left empty, then all elements are matched.
    b.is_empty()
}
