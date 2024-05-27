use bnum::types::{U256, U512};

use crate::{
    generate_grug_number, impl_bytable_bnum, impl_bytable_std, impl_checked_ops, impl_number_bound,
    uint::{Bytable, CheckedOps, GrugNumber, Uint, UintInner},
    StdError, StdResult,
};

// Uint64
generate_grug_number!(
    name = Uint64g,
    inner_type = u64,
    min = u64::MIN,
    max = u64::MAX,
    zero = 0,
    one = 1,
    byte_len = 8,
    impl_bytable = std,
    from_uint = []
);

// Uint128
generate_grug_number!(
    name = Uint128g,
    inner_type = u128,
    min = u128::MIN,
    max = u128::MAX,
    zero = 0,
    one = 1,
    byte_len = 16,
    impl_bytable = std,
    from_uint = [Uint64g]
);

// Uint256
generate_grug_number!(
    name = Uint256g,
    inner_type = U256,
    min = U256::MIN,
    max = U256::MAX,
    zero = U256::ZERO,
    one = U256::ONE,
    byte_len = 32,
    impl_bytable = bnum,
    from_uint = [Uint64g, Uint128g]
);

// Uint512
generate_grug_number!(
    name = Uint512g,
    inner_type = U512,
    min = U512::MIN,
    max = U512::MAX,
    zero = U512::ZERO,
    one = U512::ONE,
    byte_len = 64,
    impl_bytable = bnum,
    from_uint = [Uint256g, Uint64g, Uint128g]
);

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use bnum::types::{U256, U512};

    use crate::{
        uint::{Bytable, CheckedOps},
        uint64g::{Uint256g, Uint512g},
        StdResult,
    };

    use super::{Uint128g, Uint64g};

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
        assert_eq!(Uint64g::MAX.number(), u64::MAX);
        assert_eq!(Uint64g::MIN.number(), u64::MIN);
        assert_eq!(Uint64g::ZERO.number(), 0_u64);
        assert_eq!(Uint64g::ONE.number(), 1_u64);
    }

    #[test]
    fn t2_base_ops() {
        let min = Uint64g::new(10);
        let max = Uint64g::new(20);
        assert_eq!(max + min, Uint64g::new(30));
        assert_eq!(max - min, Uint64g::new(10));
        assert_eq!(max / min, Uint64g::new(2));
        assert_eq!(max * min, Uint64g::new(200));
        assert_eq!(&max * min, Uint64g::new(200));
    }

    #[test]
    fn t3_checked() {
        let min = Uint64g::new(10);
        let max = Uint64g::new(20);
        assert_eq!(min.checked_add(max).unwrap(), Uint64g::new(30));
        assert_eq!(max.checked_sub(min).unwrap(), Uint64g::new(10));
        assert_eq!(min.checked_mul(max).unwrap(), Uint64g::new(200));
    }

    #[test]
    fn t4_assign() {
        let mut var = Uint64g::new(10);
        var += Uint64g::new(20);
        assert_eq!(var, Uint64g::new(30));

        let mut var = Uint64g::new(10);
        var -= Uint64g::new(3);
        assert_eq!(var, Uint64g::new(7));
    }

    #[test]
    fn t5_from_to() {
        let from = Uint64g::new(10);
        let to: Uint128g = from.into();
        assert_eq!(to, Uint128g::new(10));

        let from = 10_u64;
        let to: Uint128g = from.into();
        assert_eq!(to, Uint128g::new(10));

        let from = Uint128g::from_le_bytes(concat_arrays!([255_u8; 8], [0; 8]));
        let to: Uint64g = from.try_into().unwrap();
        assert_eq!(to, Uint64g::MAX);

        let from = Uint128g::from_le_bytes(concat_arrays!([255_u8; 9], [0; 7]));
        let to: StdResult<Uint64g> = from.try_into();
        to.unwrap_err();

        let from = Uint256g::from_le_bytes(concat_arrays!([255_u8; 8], [0; 24]));
        let to: Uint64g = from.try_into().unwrap();
        assert_eq!(to, Uint64g::MAX);

        let from = Uint256g::from_be_bytes(concat_arrays!([0; 24], [255_u8; 8]));
        let to: Uint64g = from.try_into().unwrap();
        assert_eq!(to, Uint64g::MAX);

        let from = Uint256g::from_le_bytes(concat_arrays!([255_u8; 8], [0; 24]));
        let to: u64 = from.try_into().unwrap();
        assert_eq!(to, u64::MAX);

        let from = Uint256g::from_le_bytes(concat_arrays!([255_u8; 9], [0; 23]));
        let to: StdResult<Uint64g> = from.try_into();
        to.unwrap_err();

        let val: Uint64g = 10_u64.into();
        assert_eq!(val, Uint64g::new(10));

        let val: u64 = val.into();
        assert_eq!(val, 10_u64);

        let val: Uint512g = Uint64g::from(100).into();
        assert_eq!(val, Uint512g::new(U512::from_le_bytes(concat_arrays!([100_u8; 1], [0; 63]))));
    }

    #[test]
    fn t6_serde() {
        let val = Uint64g::from_str("10").unwrap();
        assert_eq!(val, Uint64g::from(10));
        assert_eq!(val.to_string(), "10");

        let ser = serde_json::to_vec(&val).unwrap();
        let des: Uint64g = serde_json::from_slice(ser.as_slice()).unwrap();
        assert_eq!(val, des);

        let val = Uint256g::from_str("10").unwrap();
        assert_eq!(val, Uint256g::from(10_u128));
        assert_eq!(val, Uint256g::from(U256::from_le_bytes(concat_arrays!([10_u8; 1], [0; 31]))));
        assert_eq!(val.to_string(), "10");

        val.to_le_bytes();

        let ser = serde_json::to_vec(&val).unwrap();
        let des: Uint256g = serde_json::from_slice(ser.as_slice()).unwrap();
        assert_eq!(val, des);
    }
}
