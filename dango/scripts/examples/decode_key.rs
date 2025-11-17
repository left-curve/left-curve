use {
    dango_types::gateway::Remote,
    grug::{Addr, Binary, PrimaryKey},
    std::str::FromStr,
};

const KEY: &str =
    "AAVyb3V0ZQAU34bOl4ObxE+rsy9n8dqPdohRMaIAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC";

fn main() {
    let bytes = Binary::from_str(KEY).unwrap();

    // split off the namespace
    let (len_bytes, bytes) = bytes.split_at(2);
    let len = u16::from_be_bytes(len_bytes.try_into().unwrap());
    let key = &bytes[len as usize..];

    let parsed_key = <(Addr, Remote)>::from_slice(key).unwrap();
    println!("{parsed_key:?}");
}
