use {
    crate::{Borsh, IndexPrefix, Map, MapKey},
    borsh::BorshDeserialize,
    grug_types::{StdResult, Storage},
    std::marker::PhantomData,
};

// Note: we cannot store traits with generic functions inside `Box<dyn Index>`,
// so I pull S: Storage to a top-level
pub trait Index<T> {
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()>;
    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()>;
}

// ----------------------------------- multi index -----------------------------------

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
    // note, we collapse the ik - combining everything under the namespace - and concatenating the pk
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
    IK: MapKey,
    T: BorshDeserialize,
{
    pub fn no_prefix_raw(&self) -> IndexPrefix<IK, T, Borsh> {
        IndexPrefix::with_deserialization_functions(self.idx_namespace, &[], self.pk_namespace)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {

    use borsh::{BorshDeserialize, BorshSerialize};
    use grug_types::MockStorage;

    use crate::{Borsh, Index, IndexList, IndexedMap, MultiIndex};

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    struct Foo {
        pub name: String,
        pub non_indexed: String,
    }

    impl Foo {
        pub fn new(name: &str, non_indexed: &str) -> Self {
            Foo {
                name: name.to_string(),
                non_indexed: non_indexed.to_string(),
            }
        }
    }

    struct FooIndexes<'a> {
        pub name: MultiIndex<'a, String, Foo, u64>,
    }

    impl<'a> IndexList<Foo> for FooIndexes<'a> {
        fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<Foo>> + '_> {
            let v: Vec<&dyn Index<Foo>> = vec![&self.name];
            Box::new(v.into_iter())
        }
    }

    fn foo<'a>() -> IndexedMap<'a, u64, Foo, FooIndexes<'a>, Borsh> {
        let indexes = FooIndexes {
            name: MultiIndex::new(|_, data| data.name.clone(), "pk_namespace", "name"),
        };

        IndexedMap::new("pk_namespace", indexes)
    }

    #[test]
    fn t1() {
        let mut deps = MockStorage::new();

        let map = foo();

        map.save(&mut deps, 1, &Foo::new("bar", "1")).unwrap();
        map.save(&mut deps, 2, &Foo::new("bar", "2")).unwrap();
        map.save(&mut deps, 3, &Foo::new("bar", "3")).unwrap();
        map.save(&mut deps, 4, &Foo::new("foo", "4")).unwrap();

        map.load(&deps, 1).unwrap();

        let a = map
            .idx
            .name
            .no_prefix_raw()
            .range_raw(&mut deps, None, None, grug_types::Order::Ascending)
            .map(|val| val.unwrap())
            .collect::<Vec<_>>();

        println!("{:#?}", a);
    }
}
