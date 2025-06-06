use {
    borsh::{BorshDeserialize, BorshSerialize},
    prost::bytes::Bytes,
    std::ops::Deref,
};

#[grug::derive(Serde)]
pub struct RawTx(pub Bytes);

impl Deref for RawTx {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
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
