// Event attribute keys emitted by the state machine are prefixed by an
// underscore. Contracts are not allowed to emit event attributes whose keys are
// similarly prefixed. This prevents malicious contracts from emitting an
// attribute that impersonates state machine attributes in order to fool indexers.
pub const CONTRACT_ADDRESS_KEY: &str = "_contract_address";

// Below: IBC event attribute keys.
// For IBC events, we keep them consistent with ibc-go, which may make relayer
// itegration easier.
// E.g. instead of `_contract_address` we use `client_id`; instead of `code_hash`
// we use `client_type`.
// This also means we can't prefix these keys with an underscore like we do with
// `_contract_address`, but this is fine because IBC clients are permissioned
// (only codes approved by governance and added to the chain `Config` can be
// used to create IBC clients).

/// Attribute key representing the identifier of an IBC client.
///
/// In ibc-go, this is a string such as `07-tendermint-1`. In our case, this is
/// a contract address.
pub const CLIENT_ID_KEY: &str = "client_id";

/// Attribute key representing the type of an IBC client.
///
/// In ibc-go, this is a string such as `07-tendermint`. In out case, this is
/// the client contract's Wasm code hash.
pub const CLIENT_TYPE_KEY: &str = "client_type";
