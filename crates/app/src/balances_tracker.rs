use {
    crate::Shared, borsh::{BorshDeserialize, BorshSerialize}, grug_types::{Addr, BalanceDifference, Coin, CoinDirection, NonZero, Number, Uint256}, std::{cmp::Ordering, collections::HashMap, str::FromStr},
};

const NOT_TRACKED_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(BorshSerialize, BorshDeserialize)]
struct Moves {
    credit: Uint256,
    debit: Uint256,
}

struct BalancesTrackerInner {
    tracked_address: Addr,
    denom_moves: HashMap<String, Moves>,
}

#[derive(Clone)]
pub struct BalancesTracker {
    inner: Shared<BalancesTrackerInner>,
}

impl BalancesTracker {
    /// Create a new BalancesTacker for a one address.
    pub fn new(tracked_address: Addr) -> Self {
        Self {
            inner: Shared::new(BalancesTrackerInner{
                tracked_address,
                denom_moves: HashMap::new(),
            })
        }
    }

    /// Create a new BalancesTracker for a default address,
    /// it will track nothing.
    pub fn notrack() -> Self{
        Self{
            inner: Shared::new(
                BalancesTrackerInner{
                    tracked_address: Addr::from_str(NOT_TRACKED_ADDRESS).unwrap(),
                    denom_moves:HashMap::new(),
                },
            )
        }
    }

    /// It stores new debit/credit movement for the given coin denom
    /// unless both `from` and `to` are not the tracked address.
    pub fn transfered(&self, from: &Addr, to:&Addr, denom: String, amount: Uint256) {
        self.inner.write_with(|mut inner| {
            if !inner.tracked_address.eq(&from) && !inner.tracked_address.eq(&to) {
                return;
            }
            if inner.tracked_address.eq(&from) {
                // coin goes out
                match inner.denom_moves.get_mut(&denom) {
                    None => {
                        inner.denom_moves.insert(
                        denom,
                        Moves { credit: Uint256::default(), debit:  amount});
                    },
                    Some(moves) => {
                        moves.debit = moves.debit.saturating_add(amount);
                    }
                }
            } else if inner.tracked_address.eq(&to) {
                // coin goes in
                match inner.denom_moves.get_mut(&denom) {
                    None => {
                        inner.denom_moves.insert(
                        denom,
                        Moves{credit: amount, debit: Uint256::default()});
                    }
                    Some(moves) => {
                        moves.credit = moves.credit.saturating_add(amount);
                    }
                }
            }
        });
    }

    /// It returns a vector a positive (CoinDirection::In) or negative (CoinDirection::Out) coins
    pub fn get_balances_difference(&self) -> Vec<BalanceDifference> {
        let mut diffs: Vec<BalanceDifference> = vec![];
        self.inner.read_with(|inner|{
            for (denom, moves) in inner.denom_moves.iter() {
                match moves.credit.cmp(&moves.debit) {
                    Ordering::Greater => {
                        diffs.push(BalanceDifference{
                            direction: CoinDirection::In,
                            coin: Coin::new(
                                denom,
                                NonZero::new(moves.credit.saturating_sub(moves.debit)),
                            ),
                        },
                        );
                    },
                    Ordering::Less => {
                        diffs.push(BalanceDifference{
                            direction: CoinDirection::Out,
                            coin: Coin::new(
                                denom,
                                NonZero::new(moves.debit.saturating_sub(moves.credit)),
                            ),
                        },
                        );
                    }
                    _ => {},
                }
            }
        });
        diffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_untracked() {

        let wallet1 = Addr::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let wallet2 = Addr::from_str("0x0000000000000000000000000000000000000002").unwrap();
        let tracker = BalancesTracker::notrack();
        tracker.inner.read_with(|inner|{
            assert_eq!(Addr::from_str(NOT_TRACKED_ADDRESS).unwrap(), inner.tracked_address);
            assert_eq!(0, inner.denom_moves.len());
        });

        // balance diff is +40
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin1").unwrap(), Uint256::from(10_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin1").unwrap(), Uint256::from(50_u128));
        // balance diff is -50
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin2").unwrap(), Uint256::from(60_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin2").unwrap(), Uint256::from(10_u128));
        // balance diff is 0
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin3").unwrap(), Uint256::from(70_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin3").unwrap(), Uint256::from(70_u128));

        tracker.inner.read_with(|inner|{
            assert_eq!(Addr::from_str(NOT_TRACKED_ADDRESS).unwrap(), inner.tracked_address);
            assert_eq!(0, inner.denom_moves.len());
        });

        let balances_diff = tracker.get_balances_difference();
        assert_eq!(0, balances_diff.len());
    }

    #[test]
    fn use_tracked() {
        let wallet1 = Addr::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let wallet2 = Addr::from_str("0x0000000000000000000000000000000000000002").unwrap();
        let tracker = BalancesTracker::new(wallet1.clone());
        tracker.inner.read_with(|inner|{
            assert_eq!(wallet1, inner.tracked_address);
            assert_eq!(0, inner.denom_moves.len());
        });

        // balance diff is +40
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin1").unwrap(), Uint256::from(10_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin1").unwrap(), Uint256::from(50_u128));
        // balance diff is -50
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin2").unwrap(), Uint256::from(60_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin2").unwrap(), Uint256::from(10_u128));
        // balance diff is 0
        tracker.transfered(&wallet1, &wallet2, String::from_str("coin3").unwrap(), Uint256::from(70_u128));
        tracker.transfered(&wallet2, &wallet1, String::from_str("coin3").unwrap(), Uint256::from(70_u128));

        tracker.inner.read_with(|inner|{
            assert_eq!(wallet1, inner.tracked_address);
            assert_eq!(3, inner.denom_moves.len());
            let moves = inner.denom_moves.get("coin1").unwrap();
            assert_eq!(Uint256::from(10_u128), moves.debit);
            assert_eq!(Uint256::from(50_u128), moves.credit);
            let moves = inner.denom_moves.get("coin2").unwrap();
            assert_eq!(Uint256::from(60_u128), moves.debit);
            assert_eq!(Uint256::from(10_u128), moves.credit);
            let moves = inner.denom_moves.get("coin3").unwrap();
            assert_eq!(Uint256::from(70_u128), moves.debit);
            assert_eq!(Uint256::from(70_u128), moves.credit);
        });

        let balances_diff = tracker.get_balances_difference();
        assert_eq!(2, balances_diff.len()); 
        let expected = [
            BalanceDifference{direction:CoinDirection::In, coin: Coin::new("coin1", NonZero::new(Uint256::from(40_u128)))},
            BalanceDifference{direction:CoinDirection::Out, coin: Coin::new("coin2", NonZero::new(Uint256::from(50_u128)))},
        ];
        assert!(expected.iter().all(|x| {balances_diff.contains(x)}));
    }
}
