use {
    grug::{ByteArray, Inner},
    pyth_lazer_protocol::message::LeEcdsaMessage as LazerLeEcdsaMessage,
};

#[grug::derive(Serde)]
/// LE-ECDSA format enveope.
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
