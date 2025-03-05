use crate::{
    CheckedContractEvent, EvtConfigure, EvtUpload, FlatEvent, FlatEvtAuthenticate, FlatEvtBackrun,
    FlatEvtCron, FlatEvtExecute, FlatEvtFinalize, FlatEvtGuest, FlatEvtInstantiate, FlatEvtMigrate,
    FlatEvtReply, FlatEvtTransfer, FlatEvtWithhold,
};

/// Trait that allows to convert an enum to inner value of a specific variant.
pub trait AsVariant<V> {
    fn maybe_variant(self) -> Option<V>;
}

macro_rules! impl_as_variant {
    ($enum:ident, $variant:ident => $flat_variant:ident) => {
        impl AsVariant<$flat_variant> for $enum {
            fn maybe_variant(self) -> Option<$flat_variant> {
                if let $enum::$variant(inner) = self {
                    Some(inner)
                } else {
                    None
                }
            }
        }
    };
    ($enum:ident, $($variant:ident => $flat_variant:ident),+ $(,)?) => {
        $(
            impl_as_variant! { $enum, $variant => $flat_variant }
        )*
    };
}

impl_as_variant! {
    FlatEvent,
    Configure     => EvtConfigure,
    Transfer      => FlatEvtTransfer,
    Upload        => EvtUpload,
    Instantiate   => FlatEvtInstantiate,
    Execute       => FlatEvtExecute,
    Migrate       => FlatEvtMigrate,
    Reply         => FlatEvtReply,
    Authenticate  => FlatEvtAuthenticate,
    Backrun       => FlatEvtBackrun,
    Withhold      => FlatEvtWithhold,
    Finalize      => FlatEvtFinalize,
    Cron          => FlatEvtCron,
    Guest         => FlatEvtGuest,
    ContractEvent => CheckedContractEvent,
}
