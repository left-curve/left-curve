mod map;
mod multi;
mod unique;

pub use {map::*, multi::*, unique::*};

/// Internal enum allowing indexer functions to return either a single index key
/// or multiple index keys. Used by both `UniqueIndex` and `MultiIndex`.
enum Indexer<PK, IK, T> {
    Single(fn(&PK, &T) -> IK),
    Multi(fn(&PK, &T) -> Vec<IK>),
}
