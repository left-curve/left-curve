/// A storage key as raw bytes.
///
/// ## Note
///
/// We choose not to use `Cow<[u8]>` because while it includes `Vec<u8>` and `&[u8]`,
/// it doesn't include fixed length arrays `[u8; N]`. For these they need to be
/// converted to vectors which is slow, being a heap allocation. (We can't take
/// reference of them in which case we get the "cannot return reference to
/// temporary value" error.)
pub enum RawKey<'a> {
    Owned(Vec<u8>),
    Borrowed(&'a [u8]),
    Fixed8([u8; 1]),
    Fixed16([u8; 2]),
    Fixed32([u8; 4]),
    Fixed64([u8; 8]),
    Fixed128([u8; 16]),
    Fixed256([u8; 32]),
    Fixed512([u8; 64]),
}

impl AsRef<[u8]> for RawKey<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            RawKey::Owned(v) => v,
            RawKey::Borrowed(v) => v,
            RawKey::Fixed8(v) => v,
            RawKey::Fixed16(v) => v,
            RawKey::Fixed32(v) => v,
            RawKey::Fixed64(v) => v,
            RawKey::Fixed128(v) => v,
            RawKey::Fixed256(v) => v,
            RawKey::Fixed512(v) => v,
        }
    }
}
