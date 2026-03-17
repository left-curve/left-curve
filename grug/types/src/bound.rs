use {
    crate::{Checker, Predicate, StdError, StdResult},
    grug_math::{NumberConst, Udec128},
};

/// A limit for a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound<T> {
    Inclusive(T),
    Exclusive(T),
}

/// A wrapper that enforces the value to be within the specified bounds.
pub type Bounded<T, B> = Predicate<T, B>;

macro_rules! define_bounds {
    (
        $name:ident,
        $t:ty,
        min = $min:expr,
        max = $max:expr,
        doc = $doc:expr $(,)?
    ) => {
        #[doc = $doc]
        #[derive(Debug)]
        pub struct $name;

        impl Checker<$t> for $name {
            fn check(value: &$t) -> StdResult<()> {
                let min: Option<Bound<$t>> = $min;
                let max: Option<Bound<$t>> = $max;

                match &min {
                    Some(Bound::Inclusive(bound)) if value < bound => {
                        return Err(StdError::out_of_range(value, "<", bound));
                    },
                    Some(Bound::Exclusive(bound)) if value <= bound => {
                        return Err(StdError::out_of_range(value, "<=", bound));
                    },
                    _ => (),
                }

                match &max {
                    Some(Bound::Inclusive(bound)) if value > bound => {
                        return Err(StdError::out_of_range(value, ">", bound));
                    },
                    Some(Bound::Exclusive(bound)) if value >= bound => {
                        return Err(StdError::out_of_range(value, ">=", bound));
                    },
                    _ => (),
                }

                Ok(())
            }
        }
    };
}

define_bounds! {
    ZeroInclusiveOneInclusive,
    Udec128,
    min = Some(Bound::Inclusive(Udec128::ZERO)),
    max = Some(Bound::Inclusive(Udec128::ONE)),
    doc = "Bounds checker for `[0, 1]`.",
}

define_bounds! {
    ZeroInclusiveOneExclusive,
    Udec128,
    min = Some(Bound::Inclusive(Udec128::ZERO)),
    max = Some(Bound::Exclusive(Udec128::ONE)),
    doc = "Bounds checker for `[0, 1)`.",
}

define_bounds! {
    ZeroExclusiveOneInclusive,
    Udec128,
    min = Some(Bound::Exclusive(Udec128::ZERO)),
    max = Some(Bound::Inclusive(Udec128::ONE)),
    doc = "Bounds checker for `(0, 1]`.",
}

define_bounds! {
    ZeroExclusiveOneExclusive,
    Udec128,
    min = Some(Bound::Exclusive(Udec128::ZERO)),
    max = Some(Bound::Exclusive(Udec128::ONE)),
    doc = "Bounds checker for `(0, 1)`.",
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{Inner, JsonDeExt, JsonSerExt, ResultExt, StdError},
        grug_math::{NumberConst, Udec256, Uint256},
    };

    define_bounds! {
        FeeRateBounds,
        Udec256,
        min = Some(Bound::Inclusive(Udec256::ZERO)),
        max = Some(Bound::Exclusive(Udec256::raw(
            Uint256::new_from_u128(1_000_000_000_000_000_000),
        ))),
        doc = "Bounds checker for fee rates in `[0%, 100%)`.",
    }

    type FeeRate = Bounded<Udec256, FeeRateBounds>;

    #[test]
    fn serializing_fee_rate() {
        FeeRate::new(Udec256::new_percent(0))
            .unwrap()
            .to_json_string()
            .should_succeed_and_equal("\"0\"");

        FeeRate::new(Udec256::new_percent(50))
            .unwrap()
            .to_json_string()
            .should_succeed_and_equal("\"0.5\"");
    }

    #[test]
    fn deserializing_fee_rate() {
        "\"0\""
            .deserialize_json::<FeeRate>()
            .map(Inner::into_inner)
            .should_succeed_and_equal(Udec256::new_percent(0));

        "\"0.5\""
            .deserialize_json::<FeeRate>()
            .map(Inner::into_inner)
            .should_succeed_and_equal(Udec256::new_percent(50));

        "\"1\""
            .deserialize_json::<FeeRate>()
            .should_fail_with_error(StdError::out_of_range("1", ">=", "1"));
    }
}
