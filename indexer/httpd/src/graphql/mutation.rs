use {async_graphql::*, tendermint::TendermintMutation};

pub mod tendermint;

#[derive(MergedObject, Default)]
pub struct Mutation(TendermintMutation);
