use {
    grug::{ByteArray, Inner, NonEmpty},
    pyth_lazer_protocol::{message::LeEcdsaMessage as LazerLeEcdsaMessage, router::Channel},
};

pub type PythLazerId = u32;

pub type PriceUpdate = NonEmpty<Vec<LeEcdsaMessage>>;

#[grug::derive(Serde)]
pub struct PythLazerSubscriptionDetails {
    pub id: PythLazerId,
    pub channel: Channel,
}

#[grug::derive(Serde, Borsh)]
/// LE-ECDSA format envelope.
pub struct LeEcdsaMessage {
    pub payload: Vec<u8>,
    pub signature: ByteArray<64>,
    pub recovery_id: u8,
}

impl From<LeEcdsaMessage> for LazerLeEcdsaMessage {
    fn from(message: LeEcdsaMessage) -> Self {
        LazerLeEcdsaMessage {
            payload: message.payload,
            signature: message.signature.into_inner(),
            recovery_id: message.recovery_id,
        }
    }
}

impl From<LazerLeEcdsaMessage> for LeEcdsaMessage {
    fn from(message: LazerLeEcdsaMessage) -> Self {
        LeEcdsaMessage {
            payload: message.payload,
            signature: message.signature.into(),
            recovery_id: message.recovery_id,
        }
    }
}
