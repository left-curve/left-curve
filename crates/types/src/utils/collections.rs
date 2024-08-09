/// Builds a [`BTreeMap`](std::collections::BTreeMap) with the given key-value
/// pairs.
#[macro_export]
macro_rules! btreemap {
    ($($key:expr => $value:expr),* $(,)?) => {{
        ::std::collections::BTreeMap::from([
            $(($key, $value),)*
        ])
    }};
}

/// Builds a [`HashMap`](std::collections::HashMap) with the given key-value
/// pairs.
#[macro_export]
macro_rules! hashmap {
    ($($key:expr => $value:expr),* $(,)?) => {{
        ::std::collections::HashMap::from([
            $(($key, $value),)*
        ])
    }};
}

/// Builds a [`BTreeSet`](std::collections::BTreeSet) with the given items.
#[macro_export]
macro_rules! btreeset {
    ($($element:expr),* $(,)?) => {{
        ::std::collections::BTreeSet::from([
            $($element,)*
        ])
    }};
}

/// Builds a [`HashSet`](std::collections::HashSet) with the given items.
#[macro_export]
macro_rules! hashset {
    ($($element:expr),* $(,)?) => {{
        ::std::collections::HashSet::from([
            $($element,)*
        ])
    }};
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{btreemap, btreeset};

    #[test]
    fn btreemap_macro_works() {
        let map = btreemap! {
            "larry" => "engineer",
            "jake"  => "shepherd",
        };

        assert_eq!(map, [("larry", "engineer"), ("jake", "shepherd")].into());
    }

    #[test]
    fn btreeset_macro_works() {
        let set = btreeset! { "larry", "jake" };

        assert_eq!(set, ["larry", "jake"].into());
    }
}
