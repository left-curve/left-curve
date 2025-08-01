use {
    crate::{Borsh, Codec, Path},
    std::ops::Deref,
};

pub struct Item<'a, T, C = Borsh>
where
    C: Codec<T>,
{
    path: Path<'a, T, C>,
}

impl<'a, T, C> Item<'a, T, C>
where
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str) -> Self {
        Self {
            path: Path::from_raw(storage_key.as_bytes()),
        }
    }

    pub fn path(&self) -> &Path<'a, T, C> {
        &self.path
    }
}

// `Item` is effectively a wrapper over a `Path`, so instead of implementing
// methods (`load`, `save`, ...) manually, we simply implement `Deref<Target = Path>`
// so that users can access those methods on `Path`.
impl<'a, T, C: Codec<T>> Deref for Item<'a, T, C> {
    type Target = Path<'a, T, C>;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

// ----------------------------------- test ------------------------------------

#[cfg(test)]
mod test {
    use {
        super::Item,
        borsh::{BorshDeserialize, BorshSerialize},
        grug_math::{MathError, Number, NumberConst, Uint128},
        grug_types::{MockStorage, StdError, StdResult},
        grug_types_base::BacktracedError,
    };

    #[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
    struct Config {
        pub owner: String,
        pub max_tokens: i32,
    }

    const CONFIG: Item<Config> = Item::new("config");

    #[test]
    fn save_and_load_works() {
        let mut storage = MockStorage::new();

        // Attempt to read before the data is saved.
        {
            assert!(CONFIG.load(&storage).is_err());
            assert_eq!(CONFIG.may_load(&storage).unwrap(), None);
        }

        // Attempt to read after saving the data.
        {
            let cfg = Config {
                owner: "admin".to_string(),
                max_tokens: 1234,
            };

            CONFIG.save(&mut storage, &cfg).unwrap();

            assert_eq!(CONFIG.load(&storage).unwrap(), cfg);
            assert_eq!(CONFIG.may_load(&storage).unwrap(), Some(cfg));
        }
    }

    #[test]
    fn exists_works() {
        let mut storage = MockStorage::new();

        // Call `exists` before the data is saved.
        {
            assert!(!CONFIG.exists(&storage));
        }

        // Save the data, then call `exists`.
        {
            let cfg = Config {
                owner: "admin".to_string(),
                max_tokens: 1234,
            };

            CONFIG.save(&mut storage, &cfg).unwrap();

            assert!(CONFIG.exists(&storage));
        }

        // Should be able to distinguish a data that doesn't exist, from a data
        // that exists but is `None`.
        {
            const OPTIONAL: Item<Option<u32>> = Item::new("optional");

            assert!(!OPTIONAL.exists(&storage));

            OPTIONAL.save(&mut storage, &None).unwrap();

            assert!(OPTIONAL.exists(&storage));
        }
    }

    #[test]
    fn remove_works() {
        let mut storage = MockStorage::new();

        // Save data
        {
            let cfg = Config {
                owner: "admin".to_string(),
                max_tokens: 1234,
            };

            CONFIG.save(&mut storage, &cfg).unwrap();

            assert_eq!(cfg, CONFIG.load(&storage).unwrap());
        }

        // Remove it and loads `None`.
        {
            CONFIG.remove(&mut storage);
            assert!(!CONFIG.exists(&storage));
        }

        // Safe to remove it twice.
        {
            CONFIG.remove(&mut storage);
            assert!(!CONFIG.exists(&storage));
        }
    }

    #[test]
    fn update_works() {
        let mut storage = MockStorage::new();

        // Save a new data using `update`.
        {
            let output = CONFIG
                .may_modify(&mut storage, |c| -> StdResult<_> {
                    assert!(c.is_none());

                    Ok(Some(Config {
                        owner: "admin".to_string(),
                        max_tokens: 1234,
                    }))
                })
                .unwrap();

            assert_eq!(CONFIG.may_load(&storage).unwrap(), output);
        }

        // Update the existing data using `update`.
        {
            let output = CONFIG
                .may_modify(&mut storage, |mut c| -> StdResult<_> {
                    c.as_mut().unwrap().max_tokens *= 2;

                    Ok(c)
                })
                .unwrap();

            let expected = Config {
                owner: "admin".to_string(),
                max_tokens: 2468,
            };

            assert_eq!(output.unwrap(), expected);
            assert_eq!(CONFIG.load(&storage).unwrap(), expected);
        }

        // Remove the existing data using `update`.
        {
            let output = CONFIG
                .may_modify(&mut storage, |_| -> StdResult<_> { Ok(None) })
                .unwrap();

            assert_eq!(output, None);
            assert_eq!(CONFIG.may_load(&storage).unwrap(), None);
        }
    }

    #[test]
    fn update_can_change_variable_from_outer_scope() {
        let mut storage = MockStorage::new();

        let cfg = Config {
            owner: "admin".to_string(),
            max_tokens: 1234,
        };

        CONFIG.save(&mut storage, &cfg).unwrap();

        let mut old_max_tokens = 0;

        CONFIG
            .may_modify(&mut storage, |mut c| -> StdResult<_> {
                old_max_tokens = c.as_ref().unwrap().max_tokens;

                c.as_mut().unwrap().max_tokens *= 2;

                Ok(c)
            })
            .unwrap();

        assert_eq!(old_max_tokens, 1234);
    }

    #[test]
    fn update_does_not_change_data_on_error() {
        let mut storage = MockStorage::new();

        let cfg = Config {
            owner: "admin".to_string(),
            max_tokens: 1234,
        };

        CONFIG.save(&mut storage, &cfg).unwrap();

        let res = CONFIG.may_modify(&mut storage, |_| {
            // Intentionally cause an error.
            Uint128::ONE.checked_div(Uint128::ZERO)?;

            Ok(None)
        });

        assert!(matches!(
            res,
            Err(StdError::Math(BacktracedError { error: MathError::DivisionByZero { a, .. }, .. })) if a == "1"
        ));
        assert_eq!(CONFIG.load(&storage).unwrap(), cfg);
    }

    #[test]
    fn update_supports_custom_errors() {
        #[derive(Debug)]
        enum MyError {
            Std,
            Foo,
        }

        impl From<StdError> for MyError {
            fn from(_std_error: StdError) -> MyError {
                MyError::Std
            }
        }

        let mut storage = MockStorage::new();

        let cfg = Config {
            owner: "admin".to_string(),
            max_tokens: 1234,
        };

        CONFIG.save(&mut storage, &cfg).unwrap();

        let res = CONFIG.may_modify(&mut storage, |mut c| {
            // This should emit the custom error.
            if c.as_ref().unwrap().max_tokens > 20 {
                return Err(MyError::Foo);
            }

            c.as_mut().unwrap().max_tokens += 20;

            Ok(c)
        });

        assert!(matches!(res, Err(MyError::Foo)));
        assert_eq!(CONFIG.load(&storage).unwrap(), cfg);
    }
}
