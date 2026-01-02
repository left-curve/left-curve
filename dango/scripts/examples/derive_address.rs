use {
    dango_types::{account_factory::NewUserSalt, auth::Key},
    grug::{Addr, Hash256, addr, hash},
    hex_literal::hex,
};

const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

const SINGLE_SIG_CODE_HASH: Hash256 =
    hash!("D86E8112F3C4C4442126F8E9F44F16867DA487F29052BF91B810457DB34209A4");

const PUBLIC_KEY: [u8; 33] =
    hex!("036523e6658cd6ef402feafb43b6587a47b3762397831e58209335c89d66d513ea");

const PUBLIC_KEY_HASH: Hash256 =
    hash!("4682F5BE77499C54F63BDEF6734F8E388275342BFD845322D9377A5C312B7F8B");

const SEED: u32 = 0;

fn main() {
    let address = Addr::derive(
        ACCOUNT_FACTORY,
        SINGLE_SIG_CODE_HASH,
        &NewUserSalt {
            key: Key::Secp256k1(PUBLIC_KEY.into()),
            key_hash: PUBLIC_KEY_HASH,
            seed: SEED,
        }
        .to_bytes(),
    );

    println!("{address}");
}
