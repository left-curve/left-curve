use {
    crate::{context::Context, ctx},
    borsh::{BorshDeserialize, BorshSerialize},
    malachitebft_core_types::{NilOrVal, Round, SignedExtension, VoteType},
    std::io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BNilOrVal<Value>(pub NilOrVal<Value>);

impl<Value: BorshSerialize> BorshSerialize for BNilOrVal<Value> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match &self.0 {
            NilOrVal::Nil => 0u8.serialize(writer),
            NilOrVal::Val(v) => {
                1u8.serialize(writer)?;
                v.serialize(writer)
            },
        }
    }
}

impl<Value: BorshDeserialize> BorshDeserialize for BNilOrVal<Value> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let discriminant = u8::deserialize_reader(reader)?;
        match discriminant {
            0 => Ok(BNilOrVal(NilOrVal::Nil)),
            1 => {
                let value = Value::deserialize_reader(reader)?;
                Ok(BNilOrVal(NilOrVal::Val(value)))
            },
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid discriminant: {}", discriminant),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BVoteType(pub VoteType);

impl BorshSerialize for BVoteType {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self.0 {
            VoteType::Prevote => 0u8.serialize(writer),
            VoteType::Precommit => 1u8.serialize(writer),
        }
    }
}

impl BorshDeserialize for BVoteType {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let discriminant = u8::deserialize_reader(reader)?;
        match discriminant {
            0 => Ok(BVoteType(VoteType::Prevote)),
            1 => Ok(BVoteType(VoteType::Precommit)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid discriminant: {}", discriminant),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSignedExtension(pub SignedExtension<Context>);

impl BorshSerialize for BSignedExtension {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.message.serialize(writer)?;
        self.0.signature.serialize(writer)
    }
}

impl BorshDeserialize for BSignedExtension {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let message = <ctx!(Extension)>::deserialize_reader(reader)?;
        let signature = <ctx!(SigningScheme::Signature)>::deserialize_reader(reader)?;
        Ok(BSignedExtension(SignedExtension::new(message, signature)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BRound(pub Round);

impl BorshSerialize for BRound {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self.0 {
            Round::Nil => 0u8.serialize(writer),
            Round::Some(r) => {
                1u8.serialize(writer)?;
                r.serialize(writer)
            },
        }
    }
}

impl BorshDeserialize for BRound {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let discriminant = u8::deserialize_reader(reader)?;
        match discriminant {
            0 => Ok(BRound(Round::Nil)),
            1 => {
                let r = u32::deserialize_reader(reader)?;
                Ok(BRound(Round::Some(r)))
            },
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid discriminant: {}", discriminant),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{BorshDeExt, BorshSerExt},
    };

    #[test]
    fn test_serialize_deserialize() {
        let nil_or_val = BNilOrVal::<String>(NilOrVal::Nil);
        let serialized = nil_or_val.to_borsh_vec().unwrap();
        let deserialized: BNilOrVal<String> = serialized.deserialize_borsh().unwrap();
        assert_eq!(nil_or_val, deserialized);

        let nil_or_val = BNilOrVal::<String>(NilOrVal::Val("test".to_string()));
        let serialized = nil_or_val.to_borsh_vec().unwrap();
        let deserialized: BNilOrVal<String> = serialized.deserialize_borsh().unwrap();
        assert_eq!(nil_or_val, deserialized);
    }
}
