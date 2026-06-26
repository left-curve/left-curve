//! The activity projection's GraphQL **read surface** — the layer that turns
//! the three storage tables into queryable feeds. Three files:
//!
//! - [`query`] — `ActivityQuery`, the resolvers running the eight documented
//!   feeds (see `DESIGN.md` § Read surface);
//! - [`types`] — the GraphQL type surface: the `Address` / `Hash` scalars, the
//!   `Transaction` / `Event` objects, the `UnitKind` / `EventType` enums;
//! - [`pagination`] — the keyset machinery: opaque cursors, the parameter
//!   binder, and the forward Relay-connection assembly.
//!
//! Only [`ActivityQuery`] escapes the module; everything else is plumbing the
//! composition root never names (the output types reach the schema through the
//! resolver signatures, so async-graphql registers them regardless).

mod pagination;
mod query;
mod types;

pub use query::ActivityQuery;
