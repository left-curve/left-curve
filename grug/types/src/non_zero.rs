use {
    crate::{Checker, Predicate, StdError, StdResult},
    grug_math::IsZero,
};

/// Checker that rejects zero values.
pub struct IsNonZero;

impl<T: IsZero> Checker<T> for IsNonZero {
    fn check(value: &T) -> StdResult<()> {
        if value.is_zero() {
            return Err(StdError::zero_value::<T>());
        }

        Ok(())
    }
}

/// A wrapper over a number that ensures it is non-zero.
pub type NonZero<T> = Predicate<T, IsNonZero>;

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{BorshDeExt, BorshSerExt, JsonDeExt, NonZero, StdError},
        borsh::{BorshDeserialize, BorshSerialize},
        grug_math::{NumberConst, Uint128},
    };

    // The expect error is a `StdError::Deserialize` where the `reason` is a
    // `StdError::ZeroValue`.
    fn assert_is_non_zero_err<T>(err: StdError) {
        assert!(matches!(
            err,
            StdError::Deserialize { reason, .. } if reason == StdError::zero_value::<T>().to_string()
        ));
    }

    #[test]
    fn deserializing_from_json() {
        let res = "123".deserialize_json::<NonZero<u32>>().unwrap();
        assert_eq!(res, NonZero::new_unchecked(123));

        let err = "0".deserialize_json::<NonZero<u32>>().unwrap_err();
        assert_is_non_zero_err::<u32>(err);

        let res = "\"123\"".deserialize_json::<NonZero<Uint128>>().unwrap();
        assert_eq!(res, NonZero::new_unchecked(Uint128::new(123)));

        let err = "\"0\"".deserialize_json::<NonZero<Uint128>>().unwrap_err();
        assert_is_non_zero_err::<Uint128>(err);
    }

    #[test]
    fn deserialize_from_borsh() {
        let good = NonZero::new_unchecked(Uint128::new(123))
            .to_borsh_vec()
            .unwrap();
        let res = good.deserialize_borsh::<NonZero<Uint128>>().unwrap();
        assert_eq!(res, NonZero::new_unchecked(Uint128::new(123)));

        // With the Predicate abstraction, borsh deserialization skips validation.
        // So deserializing a zero value from borsh succeeds (unlike serde).
        let bad = NonZero::new_unchecked(Uint128::ZERO)
            .to_borsh_vec()
            .unwrap();
        let res = bad.deserialize_borsh::<NonZero<Uint128>>().unwrap();
        assert_eq!(res, NonZero::new_unchecked(Uint128::ZERO));
    }

    #[test]
    fn deserializing_from_borsh_as_part_of_struct() {
        #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
        struct Data {
            number: NonZero<u32>,
            duration: u128,
        }

        let data = Data {
            number: NonZero::new(123).unwrap(),
            duration: 123,
        };

        let ser = data.to_borsh_vec().unwrap();
        let de: Data = ser.deserialize_borsh().unwrap();

        assert_eq!(de, data);
    }
}
