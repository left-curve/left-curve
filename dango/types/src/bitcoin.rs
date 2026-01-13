use {
    crate::gateway::bridge::BridgeMsg,
    anyhow::bail,
    borsh::{BorshDeserialize, BorshSerialize},
    corepc_client::bitcoin::{
        Address, Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction as BtcTransaction,
        TxIn, TxOut, Txid, Witness,
        absolute::LockTime,
        hashes::Hash,
        opcodes::all::{OP_CHECKMULTISIG, OP_DROP},
        script::Builder,
        transaction::Version,
    },
    grug::{
        Addr, Binary, BorshSerExt, Hash256, HashExt, HexBinary, HexByteArray, Inner, NonEmpty,
        Order, PrimaryKey, RawKey, StdError, StdResult, Uint128,
    },
    serde::{Deserialize, Serialize, Serializer},
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
    },
};

pub use corepc_client::bitcoin::Network;

/// Size of one signature in bytes for P2WSH.
pub const INPUT_SIGNATURES_OVERHEAD: Uint128 = Uint128::new(28);
pub const SIGNATURE_SIZE: Uint128 = Uint128::new(20);
pub const OUTPUT_SIZE: Uint128 = Uint128::new(34);

/// A Bitcoin address. This is a string representation of the address, which can be in
/// all kinds of formats. It's validated inside the contract since it depends on the network.
pub type BitcoinAddress = String;

/// An Bitcoin signature.
pub type BitcoinSignature = HexBinary;

/// The index of the output in a Bitcoin transaction.
pub type Vout = u32;

/// The recipient of a Bitcoin transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub enum Recipient {
    Vault,
    Index(u64),
    Address(Addr),
}

impl PrimaryKey for Recipient {
    type Output = Recipient;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 2;

    fn raw_keys(&self) -> Vec<RawKey<'_>> {
        match self {
            Recipient::Vault => vec![RawKey::Fixed8([0])],
            Recipient::Index(index) => vec![
                RawKey::Fixed8([1]),
                RawKey::Owned(index.to_be_bytes().to_vec()),
            ],
            Recipient::Address(addr) => {
                vec![RawKey::Fixed8([2]), RawKey::Owned(addr.inner().to_vec())]
            },
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (tag, rest) = bytes.split_at(1);
        match tag {
            [0] => Ok(Recipient::Vault),
            [1] => {
                let index = u64::from_be_bytes(rest.try_into()?);
                Ok(Recipient::Index(index))
            },
            [2] => {
                let addr = Addr::from_slice(rest)?;
                Ok(Recipient::Address(addr))
            },
            tag => Err(StdError::deserialize::<Self::Output, _, Binary>(
                "key",
                format!("unknown tag: {tag:?}"),
                bytes.into(),
            )),
        }
    }
}

/// Multisig settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultisigSettings {
    threshold: u8,
    pub_keys: NonEmpty<BTreeSet<HexByteArray<33>>>,
}

impl MultisigSettings {
    pub fn new(
        threshold: u8,
        pub_keys: NonEmpty<BTreeSet<HexByteArray<33>>>,
    ) -> anyhow::Result<Self> {
        if threshold < 1 || threshold > pub_keys.len() as u8 {
            bail!(
                "Invalid multisig parameters: threshold = {}, pub_keys = {}",
                threshold,
                pub_keys.len()
            );
        }

        // Since the public keys will not change, we validate them once during the initialization on the contract
        // instead of validating them every time we use them.
        for pubkey in pub_keys.iter() {
            PublicKey::from_slice(pubkey)?;
        }

        Ok(Self {
            threshold,
            pub_keys,
        })
    }

    /// Returns the threshold number of signatures required to authorize a transaction.
    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    /// Returns the public keys of the guardians in the multisig wallet.
    pub fn pub_keys(&self) -> NonEmpty<BTreeSet<PublicKey>> {
        // Safe to unwrap since we validated the public keys during initialization.
        let pub_keys = self
            .pub_keys
            .iter()
            .map(|pk| PublicKey::from_slice(pk).unwrap())
            .collect::<BTreeSet<_>>();

        // Safe to unwrap since we have a NonEmpty set.
        NonEmpty::new(pub_keys).unwrap()
    }

    /// Returns the public keys of the guardians in the multisig wallet as hex byte arrays.
    pub fn pub_keys_as_bytes_array(&self) -> &NonEmpty<BTreeSet<HexByteArray<33>>> {
        &self.pub_keys
    }
}

/// This represents a multisig wallet with a optional index in the witnesses script.
/// This is used to generate unique addresses for users by appending the index and OP_DROP at
/// the end of a standard multisig script.
pub struct MultisigWallet {
    script: ScriptBuf,
}

impl MultisigWallet {
    pub fn new(multisig_settings: &MultisigSettings, recipient: &Recipient) -> Self {
        // Create the script for the multisig.
        // The redeem script is a P2WSH script is created as:
        // threshold - pubkeys - num_pub_keys - OP_CHECKMULTISIG.
        // To generate unique addresses per user, we append the index and OP_DROP (if provided).
        let mut builder = Builder::new().push_int(multisig_settings.threshold() as i64);

        let pub_keys = multisig_settings.pub_keys();

        for pubkey in pub_keys.iter() {
            builder = builder.push_key(pubkey);
        }

        builder = builder
            .push_int(pub_keys.len() as i64)
            .push_opcode(OP_CHECKMULTISIG);

        // Append the index if provided.
        match recipient {
            Recipient::Vault => {},
            Recipient::Index(index) => {
                builder = builder.push_int(*index as i64).push_opcode(OP_DROP);
            },
            Recipient::Address(addr) => {
                builder = builder.push_slice(addr.into_inner()).push_opcode(OP_DROP);
            },
        }

        Self {
            script: builder.into_script(),
        }
    }

    /// Returns the Bitcoin address of the multisig wallet.
    pub fn address(&self, network: Network) -> Address {
        Address::p2wsh(&self.script, network)
    }

    /// Returns the script of the multisig wallet.
    pub fn script(&self) -> &ScriptBuf {
        &self.script
    }
}

#[grug::derive(Serde)]
pub struct Config {
    /// The Bitcoin network the bridge is operating on.
    pub network: Network,
    /// The vault address where changes are accumulated.
    pub vault: BitcoinAddress,
    /// The multisig settings.
    pub multisig: MultisigSettings,
    /// The amount of Sats for each vByte to calculate the fee.
    pub sats_per_vbyte: Uint128,
    /// The address of the fee rate updater.
    pub fee_rate_updater: Addr,
    /// Minimum amount of Sats.
    /// All deposits lower than this amount will be ignored.
    pub minimum_deposit: Uint128,
    /// The maximum number of outputs in a single transaction.
    pub max_output_per_tx: usize,
    /// Minimum amount of Sats required for a withdrawal.
    /// All withdrawal requests lower than this amount will be rejected.
    /// This is to avoid dust outputs.
    pub min_withdrawal: Uint128,
}

#[grug::derive(Serde, Borsh)]
pub struct Transaction {
    #[serde(
        serialize_with = "serialize_inputs",
        deserialize_with = "deserialize_inputs"
    )]
    /// The inputs of the transaction. The inputs are ordered by (transaction_hash, vout) so
    /// that the transaction can be reconstructed deterministically.
    pub inputs: BTreeMap<(Hash256, Vout), (Uint128, Recipient)>,
    /// The outputs of the transaction.
    pub outputs: BTreeMap<BitcoinAddress, Uint128>,
    /// The fee of the transaction.
    pub fee: Uint128,
    /// The replace identifier of the transaction.
    /// If this is set, it means that this transaction is a RBF replacement of the specified transaction.
    pub replace: Option<u32>,
}

fn serialize_inputs<S>(
    inputs: &BTreeMap<(Hash256, Vout), (Uint128, Recipient)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let converted: BTreeMap<String, (&Uint128, &Recipient)> = inputs
        .iter()
        .map(|((hash, vout), (amount, recipient))| (format!("{hash}/{vout}"), (amount, recipient)))
        .collect();
    serde::Serialize::serialize(&converted, serializer)
}

fn deserialize_inputs<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<(Hash256, Vout), (Uint128, Recipient)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: BTreeMap<String, (Uint128, Recipient)> =
        serde::Deserialize::deserialize(deserializer)?;
    map.into_iter()
        .map(|(key, (amount, recipient))| {
            let parts: Vec<&str> = key.split('/').collect();
            if parts.len() != 2 {
                return Err(serde::de::Error::custom("invalid input key format"));
            }
            let hash = Hash256::from_str(parts[0]).map_err(serde::de::Error::custom)?;
            let vout = parts[1].parse::<Vout>().map_err(serde::de::Error::custom)?;
            Ok(((hash, vout), (amount, recipient)))
        })
        .collect()
}

/// Creates a Bitcoin transaction input from the given transaction hash and vout.
pub fn create_tx_in(hash: &Hash256, vout: Vout) -> TxIn {
    let outpoint = OutPoint {
        txid: Txid::from_byte_array(hash.into_inner()),
        vout,
    };

    TxIn {
        previous_output: outpoint,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::default(),
    }
}

impl Transaction {
    /// Converts the object into a Bitcoin transaction.
    pub fn to_btc_transaction(&self, network: Network) -> anyhow::Result<BtcTransaction> {
        let inputs = self
            .inputs
            .iter()
            .map(|((hash, vout), _)| create_tx_in(hash, *vout))
            .collect::<Vec<_>>();

        let outputs = self
            .outputs
            .iter()
            .map(|(address, amount)| {
                let script = Address::from_str(address)?
                    .require_network(network)?
                    .script_pubkey();
                Ok(TxOut {
                    value: Amount::from_sat(amount.into_inner() as u64),
                    script_pubkey: script,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(BtcTransaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        })
    }
}

#[grug::derive(Serde)]
pub struct Utxo {
    pub transaction_hash: Hash256,
    pub vout: Vout,
    pub amount: Uint128,
}

#[grug::derive(Serde, Borsh)]
pub struct InboundMsg {
    /// The Bitcoin transaction hash.
    pub transaction_hash: Hash256,
    /// The transaction's output index.
    pub vout: Vout,
    /// The transaction's UTXO amount.
    pub amount: Uint128,
    /// The recipient of the inbound transfer.
    pub recipient: Recipient,
    /// Pubkey of the guardian observing the inbound transaction.
    pub pub_key: HexByteArray<33>,
}

impl InboundMsg {
    pub fn hash(&self) -> StdResult<Hash256> {
        Ok(self.to_borsh_vec()?.hash256())
    }
}

#[grug::derive(Serde)]
pub struct InboundCredential {
    pub signature: HexBinary,
}

// ------------------------------- Messages -----------------------------------
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update config for bridge; can only be called by the chain owner.
    UpdateConfig {
        fee_rate_updater: Option<Addr>,
        minimum_deposit: Option<Uint128>,
        max_output_per_tx: Option<usize>,
        // TODO: Allow changing the vault address and guardian set? This requires resetting the UTXO set.
    },
    /// Replace an outbound transaction with a new one with a higher fee.
    ReplaceByFee {
        /// Identifier of the outbound transaction to replace.
        tx_id: u32,
        /// New fee rate in sats per Vbyte.
        new_fee_rate: Uint128,
    },
    /// Update the fee rate in sats per Vbyte to calculate the fee for outbound transactions.
    UpdateFeeRate(Uint128),
    /// Observe an inbound transaction.
    ///
    /// Can only be called by the guardians.
    ObserveInbound(InboundMsg),
    /// Withdraw Bitcoin request.
    ///
    /// Can be called only by gateway, which needs to burn the tokens.
    ///
    /// Outbound transactions are pushed into a queue. Every once in a while (as
    /// defined by the contract's cronjob frequency), the contract finds all
    /// withdrawals in the queue and builds a transaction.
    Bridge(BridgeMsg),
    /// Authorize an outbound transaction.
    ///
    /// Can only be called by the guardians.
    AuthorizeOutbound {
        /// Identifier of the outbound transaction.
        ///
        /// Each outbound transaction is identified by a incremental ID.
        /// This ID is generated when a user calls the `withdraw` method.
        id: u32,
        /// A Bitcoin signature authorizing the outbound transaction.
        ///
        /// Once a threshold number of signatures has been received, a worker
        /// will pick it up and broadcast the transaction on the Bitcoin network.
        signatures: Vec<BitcoinSignature>,
        /// The public key of the guardian signing the transaction.
        pub_key: HexByteArray<33>,
    },

    /// Request a bitcoin deposit address for a user.
    CreateDepositAddress {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the contract configurations.
    #[returns(Config)]
    Config {},
    /// Enumerate the UTXOs spendable by the multisig, sorted by amount.
    #[returns(Vec<Utxo>)]
    Utxos {
        start_after: Option<Utxo>,
        limit: Option<u32>,
        order: Order,
    },
    /// Enumerate pending outbound transactions in the queue.
    #[returns(BTreeMap<BitcoinAddress, Uint128>)]
    OutboundQueue {
        start_after: Option<BitcoinAddress>,
        limit: Option<u32>,
    },
    /// Query the number of outbound transactions.
    #[returns(u32)]
    CountOutboundTransactions {},
    /// Query an outbound transaction by ID.
    #[returns(Transaction)]
    OutboundTransaction { id: u32 },
    /// Enumerate all outbound transactions.
    #[returns(BTreeMap<u32, Transaction>)]
    OutboundTransactions {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    /// Query the signatures for a single outbound transactions by ID.
    #[returns(BTreeMap<HexByteArray<33>, Vec<BitcoinSignature>>)]
    OutboundSignature { id: u32 },
    /// Enumerate all signatures for all outbound transactions.
    #[returns(BTreeMap<u32, BTreeMap<Addr, BitcoinSignature>>)]
    OutboundSignatures {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    /// Query the deposit address for a user.
    #[returns(BitcoinAddress)]
    DepositAddress { address: Addr },

    /// Query the next available index for generating a new Bitcoin address.
    #[returns(u64)]
    AccountsIndex {},
}

// ------------------------------- Events --------------------------------------

/// Event indicating an inbound transaction has been observed by a threshold
/// number of guardians.
#[grug::derive(Serde)]
#[grug::event("inbound_confirmed")]
pub struct InboundConfirmed {
    pub transaction_hash: Hash256,
    pub vout: Vout,
    pub amount: Uint128,
    pub recipient: Recipient,
}

/// Event indicating an outbound transaction has been requested, pending signatures
/// from the guardians.
///
/// Guardian worker should observe this event and sign accordingly.
#[grug::derive(Serde)]
#[grug::event("outbound_requested")]
pub struct OutboundRequested {
    pub id: u32,
    pub transaction: Transaction,
}

/// Event indicating an outbound transaction has received a threshold number of
/// signatures, and is ready to be broadcasted on the Bitcoin network.
///
/// Broadcaster worker should observe this event and broadcast accordingly.
#[grug::derive(Serde)]
#[grug::event("outbound_confirmed")]
pub struct OutboundConfirmed {
    pub id: u32,
    pub transaction: Transaction,
    pub signatures: BTreeMap<HexByteArray<33>, Vec<BitcoinSignature>>,
}

#[cfg(test)]
mod test {
    use {
        crate::bitcoin::Recipient,
        grug::{Addr, Item, MockStorage},
    };

    #[test]

    fn test_example() {
        let item = Item::<Recipient>::new("recipient");

        let mut storage = MockStorage::new();

        // Vault
        let data = Recipient::Vault;
        item.save(&mut storage, &data).unwrap();

        let decoded = item.load(&storage).unwrap();
        assert_eq!(decoded, data);

        // UserIndex
        let data = Recipient::Index(4587);
        item.save(&mut storage, &data).unwrap();

        let decoded = item.load(&storage).unwrap();
        assert_eq!(decoded, data);

        // UserAddress
        let data = Recipient::Address(Addr::mock(20));
        item.save(&mut storage, &data).unwrap();

        let decoded = item.load(&storage).unwrap();
        assert_eq!(decoded, data);
    }
}
