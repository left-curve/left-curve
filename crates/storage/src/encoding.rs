/// A marker that designates encoding schemes.
pub trait Encoding {}

/// Represents the Borsh encoding scheme.
pub struct Borsh;

impl Encoding for Borsh {}

/// Represents the Protobuf encoding scheme.
pub struct Proto;

impl Encoding for Proto {}
