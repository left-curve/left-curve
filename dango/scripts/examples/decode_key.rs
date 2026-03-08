use {
    anyhow::bail,
    dango_types::gateway::Remote,
    grug::{Addr, Binary, PrimaryKey},
    std::str::FromStr,
};

const KEY: &str =
    "AAVyb3V0ZQAU34bOl4ObxE+rsy9n8dqPdohRMaIAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC";

fn main() -> anyhow::Result<()> {
    let bytes = Binary::from_str(KEY)?;

    if bytes.len() < 2 {
        bail!("key must be at least 2 bytes long to include namespace length");
    }

    // split off the namespace
    let (len_bytes, bytes) = bytes.split_at(2);
    let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
    if bytes.len() < len {
        bail!(
            "namespace length ({len}) exceeds remaining bytes ({})",
            bytes.len()
        );
    }
    let key = &bytes[len..];

    let parsed_key = <(Addr, Remote)>::from_slice(key)?;
    println!("{parsed_key:?}");

    Ok(())
}
