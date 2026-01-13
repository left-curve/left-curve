use crate::account_factory::UserIndex;

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Register the sender as a referral with the given code.
    Referral { referrer_index: UserIndex },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the referrer of a given referee.
    #[returns(UserIndex)]
    Referrer { referee_index: UserIndex },
    /// Query the number of referees a given referrer has.
    #[returns(u32)]
    RefereeCount { user: UserIndex },
}
