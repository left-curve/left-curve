use {
    crate::{
        account::{multi, single},
        account_factory::UserIndex,
    },
    grug::{Binary, PrimaryKey, RawKey, StdError, StdResult},
    paste::paste,
    std::fmt::{self, Display},
};

/// Global index of an account.
///
/// Used as salt to derive account addresses. This ensures the uniqueness of
/// account addresses.
pub type AccountIndex = u32;

/// Information of an account.
#[grug::derive(Serde, Borsh)]
pub struct Account {
    pub index: AccountIndex,
    pub params: AccountParams,
}

// ----------------------------------- type ------------------------------------

/// Types of accounts the protocol supports.
#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "async-graphql", graphql(rename_items = "snake_case"))]
#[cfg_attr(
    feature = "sea-orm",
    derive(sea_orm::EnumIter, sea_orm::DeriveActiveEnum)
)]
#[cfg_attr(
    feature = "sea-orm",
    sea_orm(rs_type = "i16", db_type = "SmallInteger")
)]
pub enum AccountType {
    /// A single-signature account.
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 0))]
    Single,
    /// A multi-signature account.
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 2))]
    Multi,
}

impl PrimaryKey for AccountType {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey<'_>> {
        let index = match self {
            AccountType::Single => 0,
            AccountType::Multi => 1,
        };
        vec![RawKey::Fixed8([index])]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match u8::from_be_bytes(bytes.try_into()?) {
            0 => Ok(Self::Single),
            1 => Ok(Self::Multi),
            i => Err(StdError::deserialize::<Self, _, Binary>(
                "index",
                format!("unknown account type index: {i}"),
                bytes.into(),
            )),
        }
    }
}

impl Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountType::Single => write!(f, "single"),
            AccountType::Multi => write!(f, "multi"),
        }
    }
}

// ---------------------------------- params -----------------------------------

/// Parameters of an account.
#[grug::derive(Serde, Borsh)]
pub enum AccountParams {
    Single(single::Params),
    Multi(multi::Params),
}

macro_rules! generate_downcast {
    ($id:ident => $ret:ty) => {
        paste! {
            pub fn [<into_$id:snake>](self) -> $ret {
                match self {
                    AccountParams::$id(value) => value,
                    _ => panic!("AccountParams is not {}", stringify!($id)),
                }
            }

            pub fn [<is_$id:snake>](&self) -> bool {
                matches!(self, AccountParams::$id(_))
            }
        }
    };
    ($($id:ident => $ret:ty),+ $(,)?) => {
        $(
            generate_downcast!($id => $ret);
        )+
    };
}

impl AccountParams {
    generate_downcast! {
        Single => single::Params,
        Multi => multi::Params,
    }

    pub fn ty(&self) -> AccountType {
        match self {
            AccountParams::Single(_) => AccountType::Single,
            AccountParams::Multi(_) => AccountType::Multi,
        }
    }

    /// Returns the owner (user index) of the account.
    ///
    /// Returns `None` for multisig accounts.
    pub fn owner(&self) -> Option<UserIndex> {
        match self {
            AccountParams::Single(params) => Some(params.owner),
            AccountParams::Multi(_) => None,
        }
    }
}

// ------------------------------- param updates -------------------------------

/// Parameter updates to an account.
///
/// Currently only multisig accounts support parameter updates.
#[grug::derive(Serde)]
pub enum AccountParamUpdates {
    Multi(multi::ParamUpdates),
}

impl AccountParamUpdates {
    pub fn ty(&self) -> AccountType {
        match self {
            AccountParamUpdates::Multi(_) => AccountType::Multi,
        }
    }
}
