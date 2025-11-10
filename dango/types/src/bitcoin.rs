use {
    crate::gateway::bridge::BridgeMsg,
    anyhow::bail,
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
        Addr, BorshSerExt, Hash256, HashExt, HexBinary, HexByteArray, Inner, NonEmpty, Order,
        StdResult, Uint128,
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

/// The index associated at a dango address for generating unique bitcoin address.
pub type AddressIndex = u64;

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
    pub fn pub_keys(&self) -> &NonEmpty<BTreeSet<HexByteArray<33>>> {
        &self.pub_keys
    }
}

/// This represents a multisig wallet with a optional index in the witnesses script.
/// This is used to generate unique addresses for users by appending the index and OP_DROP at
/// the end of a standard multisig script.
pub struct MultisigWallet {
    script: ScriptBuf,
    index: Option<u64>,
}

impl MultisigWallet {
    pub fn new(
        threshold: u8,
        pub_keys: &NonEmpty<BTreeSet<HexByteArray<33>>>,
        index: Option<u64>,
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
        // threshold - pubkeys - num_pub_keys - OP_CHECKMULTISIG.
        // To generate unique addresses per user, we append the index and OP_DROP (if provided).
        let mut builder = Builder::new().push_int(threshold as i64);

        for pubkey in pub_keys.iter() {
            builder = builder.push_key(&PublicKey::from_slice(pubkey)?);
        }

        builder = builder
            .push_int(pub_keys.len() as i64)
            .push_opcode(OP_CHECKMULTISIG);

        // Append the index if provided.
        if let Some(index) = index {
            builder = builder.push_int(index as i64).push_opcode(OP_DROP);
        }

        Ok(Self {
            script: builder.into_script(),
            index,
        })
    }

    /// Returns the Bitcoin address of the multisig wallet.
    pub fn address(&self, network: Network) -> Address {
        Address::p2wsh(&self.script, network)
    }

    /// Returns the script of the multisig wallet.
    pub fn script(&self) -> &ScriptBuf {
        &self.script
    }

    /// Returns the index of the address.
    pub fn index(&self) -> Option<u64> {
        self.index
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
}

#[grug::derive(Serde, Borsh)]
pub struct Transaction {
    #[serde(
        serialize_with = "serialize_inputs",
        deserialize_with = "deserialize_inputs"
    )]
    /// The inputs of the transaction. The inputs are ordered by (transaction_hash, vout) so
    /// that the transaction can be reconstructed deterministically.
    pub inputs: BTreeMap<(Hash256, Vout), (Uint128, Option<AddressIndex>)>,
    /// The outputs of the transaction.
    pub outputs: BTreeMap<BitcoinAddress, Uint128>,
    /// The fee of the transaction.
    pub fee: Uint128,
}

fn serialize_inputs<S>(
    inputs: &BTreeMap<(Hash256, Vout), (Uint128, Option<AddressIndex>)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let converted: BTreeMap<String, (&Uint128, &Option<AddressIndex>)> = inputs
        .iter()
        .map(|((hash, vout), (amount, recipient_index))| {
            (format!("{hash}/{vout}"), (amount, recipient_index))
        })
        .collect();
    converted.serialize(serializer)
}

fn deserialize_inputs<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<(Hash256, Vout), (Uint128, Option<AddressIndex>)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: BTreeMap<String, (Uint128, Option<AddressIndex>)> =
        BTreeMap::deserialize(deserializer)?;
    map.into_iter()
        .map(|(key, (amount, recipient_index))| {
            let parts: Vec<&str> = key.split('/').collect();
            if parts.len() != 2 {
                return Err(serde::de::Error::custom("invalid input key format"));
            }
            let hash = Hash256::from_str(parts[0]).map_err(serde::de::Error::custom)?;
            let vout = parts[1].parse::<Vout>().map_err(serde::de::Error::custom)?;
            Ok(((hash, vout), (amount, recipient_index)))
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
    /// The address index of the inbound transfer, used to identify the recipient.
    /// If `None`, the recipient is the vault.
    pub address_index: Option<u64>,
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
    pub address_index: Option<AddressIndex>,
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
