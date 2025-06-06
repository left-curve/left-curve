use {
    crate::gateway::bridge::BridgeMsg,
    anyhow::bail,
    corepc_client::bitcoin::{
        Address, Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction as BtcTransaction,
        TxIn, TxOut, Txid, Witness, absolute::LockTime, hashes::Hash,
        opcodes::all::OP_CHECKMULTISIG, script::Builder, transaction::Version,
    },
    grug::{Addr, Denom, Hash256, HexBinary, HexByteArray, Inner, NonEmpty, Order, Uint128},
    serde::{Deserialize, Serialize, Serializer},
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
        sync::LazyLock,
    },
};

pub use corepc_client::bitcoin::Network;

pub const OVERHEAD_SIZE: Uint128 = Uint128::new(11);
pub const INPUT_SIZE: Uint128 = Uint128::new(105);
pub const OUTPUT_SIZE: Uint128 = Uint128::new(43);

pub const NAMESPACE: &str = "bitcoin";

pub const SUBDENOM: &str = "satoshi";

pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked([NAMESPACE, SUBDENOM]));

/// Bitcoin address of the P2WPKH (pay to witness public key hash) type, which
/// is 20-bytes long.
// TODO: There are other types of Bitcoin addresses.
pub type BitcoinAddress = String;

/// An Bitcoin signature.
pub type BitcoinSignature = HexBinary;

/// The index of the output in a Bitcoin transaction.
pub type Vout = u32;

/// Multisig settings for the Bitcoin multisig wallet.
#[grug::derive(Serde)]
pub struct MultisigSettings {
    threshold: u8,
    pub_keys: NonEmpty<BTreeSet<HexByteArray<33>>>,
    script: ScriptBuf,
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

        // Create the script for the multisig.
        // The redeem script is a P2WSH script is created as:
        // threshold pubkeys num_pub_keys OP_CHECKMULTISIG
        let mut builder = Builder::new().push_int(threshold as i64);

        for pubkey in pub_keys.iter() {
            builder = builder.push_key(&PublicKey::from_slice(pubkey)?);
        }

        builder = builder
            .push_int(pub_keys.len() as i64)
            .push_opcode(OP_CHECKMULTISIG);

        Ok(Self {
            threshold,
            pub_keys,
            script: builder.into_script(),
        })
    }

    /// Returns the Bitcoin address of the multisig wallet.
    pub fn address(&self, network: Network) -> Address {
        Address::p2wsh(&self.script, network)
    }

    /// Returns the threshold number of signatures required to authorize a transaction.
    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    /// Returns the public keys of the guardians in the multisig wallet.
    pub fn pub_keys(&self) -> &NonEmpty<BTreeSet<HexByteArray<33>>> {
        &self.pub_keys
    }

    /// Returns the script of the multisig wallet.
    pub fn script(&self) -> &ScriptBuf {
        &self.script
    }
}

#[grug::derive(Serde)]
pub struct Config {
    pub network: Network,
    pub vault: BitcoinAddress,
    pub guardians: NonEmpty<BTreeSet<Addr>>,
    pub multisig: MultisigSettings,
    /// The amount of Sats for each vByte to calculate the fee.
    pub sats_per_vbyte: Uint128,
    /// Strategy for choosing the UTXOs as inputs for outbound transactions.
    ///
    /// During periods of high gas price on Bitcoin network, we want to minimize
    /// the number of input UTXOs to save on gas fees. To achieve this, use `Order::Descending`
    /// so that we use the biggest UTXOs first.
    ///
    /// During period of low gas price, we may want to take the opportunity to
    /// consolidate the many small UTXOs into a few big ones. To achieve this,
    /// use `Order::Ascending`.
    pub outbound_strategy: Order,
    /// Minimum amount of Sats.
    /// All deposits lower than this amount will be ignored.
    pub minimum_deposit: Uint128,
}

#[grug::derive(Serde, Borsh)]
pub struct Transaction {
    #[serde(
        serialize_with = "serialize_inputs",
        deserialize_with = "deserialize_inputs"
    )]
    pub inputs: BTreeMap<(Hash256, Vout), Uint128>,
    pub outputs: BTreeMap<BitcoinAddress, Uint128>,
    pub fee: Uint128,
}

fn serialize_inputs<S>(
    inputs: &BTreeMap<(Hash256, Vout), Uint128>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let converted: BTreeMap<String, &Uint128> = inputs
        .iter()
        .map(|((hash, vout), amount)| (format!("{}/{}", hash, vout), amount))
        .collect();
    converted.serialize(serializer)
}

fn deserialize_inputs<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<(Hash256, Vout), Uint128>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: BTreeMap<String, Uint128> = BTreeMap::deserialize(deserializer)?;
    map.into_iter()
        .map(|(key, amount)| {
            let parts: Vec<&str> = key.split('/').collect();
            if parts.len() != 2 {
                return Err(serde::de::Error::custom("invalid input key format"));
            }
            let hash = Hash256::from_str(parts[0]).map_err(serde::de::Error::custom)?;
            let vout = parts[1].parse::<Vout>().map_err(serde::de::Error::custom)?;
            Ok(((hash, vout), amount))
        })
        .collect()
}

impl Transaction {
    /// Converts the object into a Bitcoin transaction.
    pub fn to_btc_transaction(&self, network: Network) -> anyhow::Result<BtcTransaction> {
        let inputs = self
            .inputs
            .iter()
            .map(|((hash, vout), _)| {
                let outpoint = OutPoint {
                    txid: Txid::from_byte_array(hash.into_inner()),
                    vout: *vout,
                };

                TxIn {
                    previous_output: outpoint,
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::MAX,
                    witness: Witness::default(),
                }
            })
            .collect::<Vec<_>>();

        let outputs = self
            .outputs
            .iter()
            .map(|(address, amount)| {
                let script = Address::from_str(&address)?
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

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the guardian addresses and/or threshold.
    ///
    /// Can only be called by the chain owner.
    UpdateConfig {
        sats_per_vbyte: Option<Uint128>,
        outbound_strategy: Option<Order>,
        // TODO: Allow changing the vault address and guardian set? This requires resetting the UTXO set.
    },
    /// Observe an inbound transaction.
    ///
    /// Can only be called by the guardians.
    ObserveInbound {
        /// The Bitcoin transaction hash.
        transaction_hash: Hash256,
        /// The transaction's output index.
        vout: Vout,
        /// The transaction's UTXO amount.
        amount: Uint128,
        /// The recipient of the inbound transfer.
        ///
        /// In case of a user making a deposit, he must indicate the recipient
        /// address in the transaction's memo. The guardian must report this
        /// recipient.
        ///
        /// Other kinds of inbound transactions do not have a recipient. For
        /// example, an outbound transaction may have an excess amount. Or, the
        /// operator may top up the multisig's balance to cover gas cost.
        recipient: Option<Addr>,
    },
    /// Withdraw Bitcoin buy burning the synthetic token on Dango.
    ///
    /// Can be called by anyone. Caller must send a non-zero amount of synthetic
    /// Bitcoin token and nothing else.
    ///
    /// Outbound transactions are pushed into a queue. Every once in a while (as
    /// defined by the contract's cronjob frequency), the contract finds all
    /// withdrawals in the queue, and builds a transaction.
    // Withdraw {
    //     /// The recipient Bitcoin address.
    //     ///
    //     /// TODO: There are various bitcoin address formats. Should we enforce one?
    //     ///
    //     /// https://bitcoin.stackexchange.com/questions/119736
    //     recipient: BitcoinAddress,
    // },
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
    /// Query the last outbound transaction ID.
    #[returns(u32)]
    LastOutboundTransactionId {},
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
}

/// Event indicating an inbound transaction has been observed by a threshold
/// number of guardians.
#[grug::derive(Serde)]
#[grug::event("inbound_confirmed")]
pub struct InboundConfirmed {
    pub transaction_hash: Hash256,
    pub amount: Uint128,
    pub recipient: Option<Addr>,
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
