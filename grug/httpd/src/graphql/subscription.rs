use {async_graphql::*, grug::GrugSubscription};

pub mod grug;

#[derive(MergedSubscription, Default)]
pub struct Subscription(GrugSubscription);
