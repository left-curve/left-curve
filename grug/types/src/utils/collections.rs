/// Builds a [`BTreeMap`](std::collections::BTreeMap) with the given key-value pairs.
#[macro_export]
macro_rules! btree_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        ::std::collections::BTreeMap::from([
            $(($key, $value),)*
        ])
    }};
}

/// Builds a [`HashMap`](std::collections::HashMap) with the given key-value pairs.
// TODO: delete after the same macro is stablized in std: https://github.com/rust-lang/rust/issues/144032
#[macro_export]
macro_rules! hash_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        ::std::collections::HashMap::from([
            $(($key, $value),)*
        ])
    }};
}

/// Builds a [`BTreeSet`](std::collections::BTreeSet) with the given items.
#[macro_export]
macro_rules! btree_set {
    ($($element:expr),* $(,)?) => {{
        ::std::collections::BTreeSet::from([
            $($element,)*
        ])
    }};
}

/// Builds a [`HashSet`](std::collections::HashSet) with the given items.
#[macro_export]
macro_rules! hash_set {
    ($($element:expr),* $(,)?) => {{
        ::std::collections::HashSet::from([
            $($element,)*
        ])
    }};
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{btree_map, btree_set};

    #[test]
    fn btreemap_macro_works() {
        let map = btree_map! {
            "larry" => "engineer",
            "jake"  => "shepherd",
        };

        assert_eq!(map, [("larry", "engineer"), ("jake", "shepherd")].into());
    }

    #[test]
    fn btreeset_macro_works() {
        let set = btree_set! { "larry", "jake" };

        assert_eq!(set, ["larry", "jake"].into());
    }
}
