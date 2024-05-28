use bnum::types::{U256, U512};

use crate::{
    generate_int, impl_bytable_bnum, impl_bytable_std, impl_checked_ops_unsigned, impl_next,
    impl_number_bound,
    uint::{Bytable, CheckedOps, NumberConst, NextNumber, Uint, UintInner},
    StdError, StdResult,
};



// Int64
generate_int!(
    name = Int64g,
    inner_type = i64,
    min = i64::MIN,
    max = i64::MAX,
    zero = 0,
    one = 1,
    byte_len = 8,
    impl_bytable = std,
    from = []
);

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use bnum::types::{U256, U512};

    use crate::{
        uint::{Bytable, CheckedOps},
        Uint64::{Uint256, Uint512},
        StdResult,
    };

    use super::{Int64g, Uint128, Uint64};

    macro_rules! concat_arrays {
        ($arr1:expr, $arr2:expr) => {{
            const LEN1: usize = $arr1.len();
            const LEN2: usize = $arr2.len();
            const LEN: usize = LEN1 + LEN2;

            let mut result = [0; LEN];
            result[..LEN1].copy_from_slice(&$arr1);
            result[LEN1..].copy_from_slice(&$arr2);

            result
        }};
    }

    #[test]
    fn t1_bounds() {
        assert_eq!(Uint64::MAX.number(), u64::MAX);
        assert_eq!(Uint64::MIN.number(), u64::MIN);
        assert_eq!(Uint64::ZERO.number(), 0_u64);
        assert_eq!(Uint64::ONE.number(), 1_u64);
    }

    #[test]
    fn t2_base_ops() {
        let min = Uint64::new(10);
        let max = Uint64::new(20);
        assert_eq!(max + min, Uint64::new(30));
        assert_eq!(max - min, Uint64::new(10));
        assert_eq!(max / min, Uint64::new(2));
        assert_eq!(max * min, Uint64::new(200));
        assert_eq!(&max * min, Uint64::new(200));
    }

    #[test]
    fn t3_checked() {
        let min = Uint64::new(10);
        let max = Uint64::new(20);
        assert_eq!(min.checked_add(max).unwrap(), Uint64::new(30));
        assert_eq!(max.checked_sub(min).unwrap(), Uint64::new(10));
        assert_eq!(min.checked_mul(max).unwrap(), Uint64::new(200));
    }

    #[test]
    fn t4_assign() {
        let mut var = Uint64::new(10);
        var += Uint64::new(20);
        assert_eq!(var, Uint64::new(30));

        let mut var = Uint64::new(10);
        var -= Uint64::new(3);
        assert_eq!(var, Uint64::new(7));
    }

    #[test]
    fn t5_from_to() {
        let from = Uint64::new(10);
        let to: Uint128 = from.into();
        assert_eq!(to, Uint128::new(10));

        let from = 10_u64;
        let to: Uint128 = from.into();
        assert_eq!(to, Uint128::new(10));

        let from = Uint128::from_le_bytes(concat_arrays!([255_u8; 8], [0; 8]));
        let to: Uint64 = from.try_into().unwrap();
        assert_eq!(to, Uint64::MAX);

        let from = Uint128::from_le_bytes(concat_arrays!([255_u8; 9], [0; 7]));
        let to: StdResult<Uint64> = from.try_into();
        to.unwrap_err();

        let from = Uint256::from_le_bytes(concat_arrays!([255_u8; 8], [0; 24]));
        let to: Uint64 = from.try_into().unwrap();
        assert_eq!(to, Uint64::MAX);

        let from = Uint256::from_be_bytes(concat_arrays!([0; 24], [255_u8; 8]));
        let to: Uint64 = from.try_into().unwrap();
        assert_eq!(to, Uint64::MAX);

        let from = Uint256::from_le_bytes(concat_arrays!([255_u8; 8], [0; 24]));
        let to: u64 = from.try_into().unwrap();
        assert_eq!(to, u64::MAX);

        let from = Uint256::from_le_bytes(concat_arrays!([255_u8; 9], [0; 23]));
        let to: StdResult<Uint64> = from.try_into();
        to.unwrap_err();

        let val: Uint64 = 10_u64.into();
        assert_eq!(val, Uint64::new(10));

        let val: u64 = val.into();
        assert_eq!(val, 10_u64);

        let val: Uint512 = Uint64::from(100).into();
        assert_eq!(val, Uint512::new(U512::from_le_bytes(concat_arrays!([100_u8; 1], [0; 63]))));
    }

    #[test]
    fn t6_serde() {
        let val = Uint64::from_str("10").unwrap();
        assert_eq!(val, Uint64::from(10));
        assert_eq!(val.to_string(), "10");

        let ser = serde_json::to_vec(&val).unwrap();
        let des: Uint64 = serde_json::from_slice(ser.as_slice()).unwrap();
        assert_eq!(val, des);

        let val = Uint256::from_str("10").unwrap();
        assert_eq!(val, Uint256::from(10_u128));
        assert_eq!(val, Uint256::from(U256::from_le_bytes(concat_arrays!([10_u8; 1], [0; 31]))));
        assert_eq!(val.to_string(), "10");

        val.to_le_bytes();

        let ser = serde_json::to_vec(&val).unwrap();
        let des: Uint256 = serde_json::from_slice(ser.as_slice()).unwrap();
        assert_eq!(val, des);
    }

    #[test]
    fn t7_int() {
        let foo = Int64g::new(-10);
        let bar = Int64g::new(10);

        assert_eq!(foo.to_string(), "-10");
        assert_eq!(bar.to_string(), "10");

        assert_eq!(foo - bar, Int64g::new(-20));
        assert_eq!(foo * foo, Int64g::new(100));
        assert_eq!(bar / foo, Int64g::new(-1));
        assert_eq!(foo + bar, Int64g::new(0));

        let foo = Int64g::from_str("-10").unwrap();
        assert_eq!(foo, Int64g::new(-10));

        let des = serde_json::to_vec(&foo).unwrap();
        let ser = serde_json::from_slice(&des).unwrap();
        assert_eq!(foo, ser);
    }

    #[test]
    fn t8_next() {
        let foo = Uint64::MAX;
        let bar = Uint64::MAX;
        assert_eq!(foo.full_mul(bar), Uint128::from(u64::MAX as u128 * u64::MAX as u128));
    }
}
