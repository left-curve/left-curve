use {
    crate::{Borsh, IndexPrefix, Map, MapKey},
    borsh::BorshDeserialize,
    grug_types::{StdResult, Storage},
    std::marker::PhantomData,
};

pub trait Index<T> {
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()>;
}

/// MultiIndex stores (namespace, index_name, idx_value, pk) -> b"pk_len".
/// Allows many values per index, and references pk.
/// The associated primary key value is stored in the main (pk_namespace) map,
/// which stores (namespace, pk_namespace, pk) -> value.
///
/// The stored pk_len is used to recover the pk from the index namespace, and perform
/// the secondary load of the associated value from the main map.
///
/// The PK type defines the type of Primary Key, both for deserialization, and
/// more important, as the type-safe bound key type.
/// This type must match the encompassing `IndexedMap` primary key type,
/// or its owned variant.
pub struct MultiIndex<'a, IK, T, PK, E = Borsh> {
    index: fn(&[u8], &T) -> IK,
    idx_namespace: &'a [u8],
    idx_map: Map<'a, &'a [u8], u32>,
    pk_namespace: &'a [u8],
    phantom_pk: PhantomData<PK>,
    phantom_e: PhantomData<E>,
}

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK> {
    pub const fn new(
        idx_fn: fn(&[u8], &T) -> IK,
        pk_namespace: &'a str,
        idx_namespace: &'static str,
    ) -> Self {
        MultiIndex {
            index: idx_fn,
            idx_namespace: idx_namespace.as_bytes(),
            idx_map: Map::new(idx_namespace),
            pk_namespace: pk_namespace.as_bytes(),
            phantom_pk: PhantomData,
            phantom_e: PhantomData,
        }
    }
}

impl<'a, IK, T, PK> Index<T> for MultiIndex<'a, IK, T, PK>
where
    IK: MapKey,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, data).joined_extra_key(pk);
        self.idx_map.save(store, &idx, &(pk.len() as u32))
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, old_data).joined_extra_key(pk);
        self.idx_map.remove(store, &idx);
        Ok(())
    }
}

// ----------------------------------- encoding -----------------------------------

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK, Borsh>
where
    PK: MapKey,
    IK: MapKey,
    T: BorshDeserialize,
{
    pub fn no_prefix_raw(&self) -> IndexPrefix<PK, T, Borsh> {
        IndexPrefix::with_deserialization_functions(self.idx_namespace, &[], self.pk_namespace)
    }

    pub fn prefix(&self, p: IK) -> IndexPrefix<PK, T, Borsh> {
        IndexPrefix::with_deserialization_functions(
            self.idx_namespace,
            &p.raw_keys(),
            self.pk_namespace,
        )
    }

    pub fn sub_prefix(&self, p: IK::Prefix) -> IndexPrefix<PK, T, Borsh> {
        IndexPrefix::with_deserialization_functions(
            self.idx_namespace,
            &p.raw_keys(),
            self.pk_namespace,
        )
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {

    use {
        crate::{Borsh, Index, IndexList, IndexedMap, MultiIndex},
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::{MockStorage, Order, StdError, StdResult},
        std::{str::from_utf8, vec},
    };

    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Foo {
        pub name: String,
        pub surname: String,
    }

    impl Foo {
        pub fn new(name: &str, surname: &str) -> Self {
            Foo {
                name: name.to_string(),
                surname: surname.to_string(),
            }
        }
    }

    struct FooIndexes<'a> {
        pub name: MultiIndex<'a, String, Foo, u64>,
        pub name_surname: MultiIndex<'a, (String, String), Foo, u64>,
    }

    impl<'a> IndexList<Foo> for FooIndexes<'a> {
        fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<Foo>> + '_> {
            let v: Vec<&dyn Index<Foo>> = vec![&self.name, &self.name_surname];
            Box::new(v.into_iter())
        }
    }

    fn foo<'a>() -> IndexedMap<'a, u64, Foo, FooIndexes<'a>, Borsh> {
        let indexes = FooIndexes {
            name: MultiIndex::new(|_, data| data.name.clone(), "pk_namespace", "name"),
            name_surname: MultiIndex::new(
                |_, data| (data.name.clone(), data.surname.clone()),
                "pk_namespace",
                "name_surname",
            ),
        };

        IndexedMap::new("pk_namespace", indexes)
    }

    fn default<'a>() -> (MockStorage, IndexedMap<'a, u64, Foo, FooIndexes<'a>, Borsh>) {
        let mut deps = MockStorage::new();
        let map = foo();
        map.save(&mut deps, 1, &Foo::new("bar", "s_bar")).unwrap();
        map.save(&mut deps, 2, &Foo::new("bar", "s_bar")).unwrap();
        map.save(&mut deps, 3, &Foo::new("bar", "s_foo")).unwrap();
        map.save(&mut deps, 4, &Foo::new("foo", "s_foo")).unwrap();
        (deps, map)
    }

    fn _keys_to_utf8(bytes: &[u8]) -> StdResult<Vec<String>> {
        let mut bytes = bytes.to_vec();
        let mut res = vec![];

        while bytes.len() != 0 {
            let len = bytes.drain(0..2).collect::<Vec<_>>();
            let len = u16::from_be_bytes(len.try_into().unwrap());
            let decoded_key = if len > bytes.len() as u16 || len == 0 {
                let decoded_key = from_utf8(&bytes)
                    .map_err(|_| StdError::generic_err("Invalid parse to utf8"))?
                    .to_string();
                bytes.clear();
                decoded_key
            } else {
                let (raw_key, rest) = bytes.split_at(len as usize);
                let decoded_key = from_utf8(raw_key)
                    .map_err(|_| StdError::generic_err("Invalid parse to utf8"))?
                    .to_string();
                bytes = rest.to_vec();
                decoded_key
            };
            res.push(decoded_key);
        }
        dbg!(&res);

        Ok(res)
    }

    #[test]
    fn index_no_prefix() {
        let (deps, map) = default();

        let val = map
            .idx
            .name_surname
            .no_prefix_raw()
            .range_raw(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![
                (1_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar")),
                (2_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar")),
                (3_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_foo")),
                (4_u64.to_be_bytes().to_vec(), Foo::new("foo", "s_foo"))
            ]
        );

        let val = map
            .idx
            .name_surname
            .no_prefix_raw()
            .range(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![
                (1, Foo::new("bar", "s_bar")),
                (2, Foo::new("bar", "s_bar")),
                (3, Foo::new("bar", "s_foo")),
                (4, Foo::new("foo", "s_foo"))
            ]
        );
    }

    #[test]
    fn index_prefix() {
        let (deps, map) = default();

        let val = map
            .idx
            .name_surname
            .prefix(("bar".to_string(), "s_bar".to_string()))
            .range_raw(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![
                (1_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar")),
                (2_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar"))
            ]
        );

        let val = map
            .idx
            .name_surname
            .prefix(("bar".to_string(), "s_bar".to_string()))
            .range(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![(1, Foo::new("bar", "s_bar")), (2, Foo::new("bar", "s_bar")),]
        );
    }

    #[test]
    fn index_sub_prefix() {
        let (deps, map) = default();

        let val = map
            .idx
            .name_surname
            .sub_prefix("bar".to_string())
            .range_raw(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![
                (1_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar")),
                (2_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_bar")),
                (3_u64.to_be_bytes().to_vec(), Foo::new("bar", "s_foo"))
            ]
        );

        let val = map
            .idx
            .name_surname
            .sub_prefix("bar".to_string())
            .range(&deps, None, None, Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            val,
            vec![
                (1, Foo::new("bar", "s_bar")),
                (2, Foo::new("bar", "s_bar")),
                (3, Foo::new("bar", "s_foo"))
            ]
        );
    }
}
