use {
    anyhow::ensure,
    grug::{Bytable, HexBinary, Inner, NextNumber, PrevNumber, Uint128, Uint256},
    hyperlane_types::Addr32,
};

/// The message to be sent via Hyperlane mailbox.
#[derive(Debug)]
pub struct TokenMessage {
    pub recipient: Addr32,
    // Note: In Grug we use `Uint128` to represent token amounts, but the Warp
    // token message uses a 256-bit number to conform to EVM standard. Make sure
    // to account for this when encoding/decoding.
    //
    // Additinally, if someone sends a token from EVM that's more than `Uint128::MAX`,
    // it will error on the destination chain which means the token is stuck on
    // the sender chain.
    pub amount: Uint128,
    pub metadata: HexBinary,
}

impl TokenMessage {
    pub fn encode(&self) -> HexBinary {
        let mut buf = Vec::with_capacity(64 + self.metadata.len());
        buf.extend(self.recipient.inner());
        // Important: cast the amount of 256-bit.
        buf.extend(self.amount.into_next().to_be_bytes());
        buf.extend(self.metadata.inner());
        buf.into()
    }

    pub fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            buf.len() >= 64,
            "token message should be at least 64 bytes, got: {}",
            buf.len()
        );

        Ok(Self {
            recipient: Addr32::from_inner(buf[0..32].try_into().unwrap()),
            // Important: deserialize the number into 256-bit and try casting
            // into 258-bit. This can fail if the number is too large! Failing
            // here causes collateral tokens being stuck on the origin chain.
            // We should implement frontend check to prevent this.
            amount: Uint256::from_be_bytes(buf[32..64].try_into().unwrap()).checked_into_prev()?,
            metadata: buf[64..].to_vec().into(),
        })
    }
}
