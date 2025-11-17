use {
    grug::{Binary, ByteArray, Inner, NonEmpty},
    pyth_lazer_protocol::{api::Channel, message::LeEcdsaMessage as LazerLeEcdsaMessage},
};

pub type PythId = u32;

pub type PriceUpdate = NonEmpty<Vec<LeEcdsaMessage>>;

#[grug::derive(Serde)]
pub struct PythLazerSubscriptionDetails {
    pub id: PythId,
    pub channel: Channel,
}

/// LE-ECDSA format envelope.
#[grug::derive(Serde, Borsh)]
pub struct LeEcdsaMessage {
    pub payload: Binary,
    pub signature: ByteArray<64>,
    pub recovery_id: u8,
}

impl From<LeEcdsaMessage> for LazerLeEcdsaMessage {
    fn from(message: LeEcdsaMessage) -> Self {
        LazerLeEcdsaMessage {
            payload: message.payload.into_inner(),
            signature: message.signature.into_inner(),
            recovery_id: message.recovery_id,
        }
    }
}

impl From<LazerLeEcdsaMessage> for LeEcdsaMessage {
    fn from(message: LazerLeEcdsaMessage) -> Self {
        LeEcdsaMessage {
            payload: message.payload.into(),
            signature: message.signature.into(),
            recovery_id: message.recovery_id,
        }
    }
}
