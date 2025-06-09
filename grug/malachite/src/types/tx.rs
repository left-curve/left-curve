use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug::{JsonSerExt, Tx},
    prost::bytes::Bytes,
    std::ops::Deref,
};

#[grug::derive(Serde)]
pub struct RawTx(pub Bytes);

impl RawTx {
    pub fn from_tx(tx: Tx) -> anyhow::Result<Self> {
        Ok(Self(Bytes::from(tx.to_json_vec()?)))
    }

    pub fn from_bytes<B>(bytes: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self(bytes.into())
    }
}

impl Deref for RawTx {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for RawTx {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl BorshSerialize for RawTx {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.serialize(writer)
    }
}

impl BorshDeserialize for RawTx {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes = Vec::<u8>::deserialize_reader(reader)?;
        Ok(RawTx(Bytes::from(bytes)))
    }
}
