use {
    super::Username,
    crate::account::{multi, single},
    grug::{PrimaryKey, RawKey, StdError, StdResult},
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
#[derive(Copy, PartialOrd, Ord)]
pub enum AccountType {
    /// A single-signature account that cannot borrow margin loans.
    Spot,
    /// A single-signature account that can borrow margin loans.
    ///
    /// The loans are collateralized by assets held in the account. The account
    /// is capable of rejecting transactions that may cause it to become
    /// insolvent, and carrying out liquidations if necessary.
    Margin,
    /// A multi-signature account. Cannot borrow margin loans.
    Multi,
}

impl PrimaryKey for AccountType {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        let index = match self {
            AccountType::Spot => 0,
            AccountType::Margin => 1,
            AccountType::Multi => 2,
        };
        vec![RawKey::Fixed8([index])]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match u8::from_be_bytes(bytes.try_into()?) {
            0 => Ok(Self::Spot),
            1 => Ok(Self::Margin),
            2 => Ok(Self::Multi),
            i => Err(StdError::deserialize::<Self, _>(
                "index",
                format!("unknown account type index: {i}"),
            )),
        }
    }
}

impl Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountType::Spot => write!(f, "spot"),
            AccountType::Margin => write!(f, "margin"),
            AccountType::Multi => write!(f, "multi"),
        }
    }
}

// ---------------------------------- params -----------------------------------

/// Parameters of an account.
#[grug::derive(Serde, Borsh)]
pub enum AccountParams {
    Spot(single::Params),
    Margin(single::Params),
    Multi(multi::Params),
}

macro_rules! generate_downcast {
    ($id:ident => $ret:ty) => {
        paste! {
            pub fn [<as_$id:snake>](self) -> $ret {
                match self {
                    AccountParams::$id(value) => value,
                    _ => panic!("AccountParams is not {}", stringify!($id)),
                }
            }

            pub fn [<is_$id:snake>](self) -> bool {
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
        Spot   => single::Params,
        Margin => single::Params,
        Multi  => multi::Params,
    }

    pub fn ty(&self) -> AccountType {
        match self {
            AccountParams::Spot { .. } => AccountType::Spot,
            AccountParams::Margin { .. } => AccountType::Margin,
            AccountParams::Multi(_) => AccountType::Multi,
        }
    }

    /// Returns the owner (username) of the account.
    ///
    /// Returns `None` for multisig accounts.
    pub fn owner(&self) -> Option<Username> {
        match self {
            AccountParams::Spot(params) => Some(params.owner.clone()),
            AccountParams::Margin(params) => Some(params.owner.clone()),
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
