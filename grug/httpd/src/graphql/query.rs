use {async_graphql::MergedObject, grug::GrugQuery};

pub mod grug;

#[derive(MergedObject, Default)]
pub struct Query(GrugQuery);
