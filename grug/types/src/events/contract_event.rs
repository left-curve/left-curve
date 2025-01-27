use serde::Serialize;

pub trait AsContractEvent: Serialize {
    const NAME: &'static str;
}
