use bnum::types::{I256, I512, U256, U512};

use crate::{
    impl_bytable_bnum, impl_bytable_ibnum, impl_bytable_std, impl_checked_ops,
    impl_checked_ops_signed, impl_checked_ops_unsigned, impl_number_bound,
    math::macros::{grow_be_int, grow_be_uint, grow_le_int, grow_le_uint},
    Bytable, CheckedOps, NumberConst, StdError, StdResult,
};

// --- Bytable ---

impl_bytable_std!(u64, 8);
impl_bytable_std!(u128, 16);
impl_bytable_bnum!(U256, 32);
impl_bytable_bnum!(U512, 64);

impl_bytable_std!(i64, 8);
impl_bytable_std!(i128, 16);
impl_bytable_ibnum!(I256, 32, U256);
impl_bytable_ibnum!(I512, 64, U512);

// --- NumberBound ---

impl_number_bound!(u64, 0, u64::MAX, 0, 1, 10);
impl_number_bound!(u128, 0, u128::MAX, 0, 1, 10);
impl_number_bound!(U256, U256::MIN, U256::MAX, U256::ZERO, U256::ONE, U256::TEN);
impl_number_bound!(U512, U512::MIN, U512::MAX, U512::ZERO, U512::ONE, U512::TEN);

impl_number_bound!(i64, 0, i64::MAX, 0, 1, 10);
impl_number_bound!(i128, 0, i128::MAX, 0, 1, 10);
impl_number_bound!(I256, I256::MIN, I256::MAX, I256::ZERO, I256::ONE, I256::TEN);
impl_number_bound!(I512, I512::MIN, I512::MAX, I512::ZERO, I512::ONE, I512::TEN);

// --- CheckedOps ---

impl_checked_ops_unsigned!(u64);
impl_checked_ops_unsigned!(u128);
impl_checked_ops_unsigned!(U256);
impl_checked_ops_unsigned!(U512);

impl_checked_ops_signed!(i64);
impl_checked_ops_signed!(i128);
impl_checked_ops_signed!(I256);
impl_checked_ops_signed!(I512);
