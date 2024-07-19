// Event attribute keys emitted by the state machine are prefixed by an
// underscore. Contracts are not allowed to emit event attributes whose keys are
// similarly prefixed. This prevents malicious contracts from emitting an
// attribute that impersonates state machine attributes in order to fool indexers.
pub const CONTRACT_ADDRESS_KEY: &str = "_contract_address";
