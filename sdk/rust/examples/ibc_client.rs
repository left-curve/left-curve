use {
    grug_ibc_solomachine::{
        ClientState, ConsensusState, Header, Misbehavior, QueryMsg, Record, SignBytes,
        StateResponse,
    },
    grug_sdk::{Client, SigningKey, SigningOptions},
    grug::{hash, to_borsh_vec, Addr, Hash, IbcClientStatus, StdResult},
    hex_literal::hex,
    home::home_dir,
    lazy_static::lazy_static,
    std::{env, path::PathBuf, thread, time::Duration},
};

lazy_static! {
    static ref ARTIFACT_DIR: PathBuf = {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../artifacts")
    };
    static ref KEYSTORE_DIR: PathBuf = {
        home_dir().unwrap().join(".cwcli/keys")
    };
}

const USER: Addr = Addr::from_slice(hex!("5f93cc3ed709beb4d0b105d43f65818fafc943cb10adc06f4f82cce82313069d"));

const SOLOMACHINE_HASH: Hash = Hash::from_slice(hex!("33a296379077e3b2ef62d63f9db1d92bc8955643ea66b7c3a13388526f0cf39e"));

const KEYSTORE_PASSWORD: &str = "123";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load signing key
    let test1 = SigningKey::from_file(&KEYSTORE_DIR.join("test1.json"), KEYSTORE_PASSWORD)?;
    let sign_opts = SigningOptions {
        signing_key: test1.clone(),
        sender:      USER.clone(),
        chain_id:    None,
        sequence:    None,
    };

    // create client
    let client = Client::connect("http://127.0.0.1:26657")?;

    // ----------------------------- create client -----------------------------

    // create a new IBC client using the solo machine code hash
    let salt = b"06-solomachine-0".to_vec().into();
    let (address, tx1) = client.create_client(
        SOLOMACHINE_HASH,
        &ClientState {
            status: IbcClientStatus::Active,
        },
        &ConsensusState {
            public_key: test1.public_key().to_vec().into(),
            sequence: 0,
            record: None,
        },
        salt,
        &sign_opts,
    )
    .await?;
    println!("\nCreating IBC client...");
    println!("address: {}", address);
    println!("txhash: {}", tx1.hash);

    // wait 1 second for tx to settle
    thread::sleep(Duration::from_secs(1));

    // query the client's state
    query_client_state(&client, &address).await?;

    // ----------------------------- update client -----------------------------

    // sign a header and update client state
    let header = create_header(b"foo", b"bar", 0, &test1)?;
    let tx2 = client.update_client(address.clone(), &header, &sign_opts).await?;
    println!("\nUpdating IBC client...");
    println!("txhash: {}", tx2.hash);

    // wait 1 second for tx to settle
    thread::sleep(Duration::from_secs(1));

    // query the client's state again
    query_client_state(&client, &address).await?;

    // ----------------------------- freeze client -----------------------------

    // sign two headers at the same sequence and submit misbehavior
    let header_one = create_header(b"foo", b"bar", 1, &test1)?;
    let header_two = create_header(b"fuzz", b"buzz", 1, &test1)?;
    let misbehavior = Misbehavior {
        sequence: 1,
        header_one,
        header_two,
    };
    let tx3 = client.freeze_client(address.clone(), &misbehavior, &sign_opts).await?;
    println!("\nFreezing client on misbehavior...");
    println!("txhash: {}", tx3.hash);

    // wait 1 second for tx to settle
    thread::sleep(Duration::from_secs(1));

    // query the client's state again
    query_client_state(&client, &address).await?;

    Ok(())
}

async fn query_client_state(client: &Client, address: &Addr) -> anyhow::Result<()> {
    let state_res: StateResponse = client.query_wasm_smart(
        address.clone(),
        &QueryMsg::State {},
        None,
    )
    .await?;
    println!("\n{}", serde_json::to_string_pretty(&state_res)?);
    Ok(())
}

fn create_header(key: &[u8], value: &[u8], sequence: u64, sk: &SigningKey) -> StdResult<Header> {
    let record = Some(Record {
        key: key.to_vec().into(),
        value: value.to_vec().into(),
    });
    let sign_bytes = SignBytes {
        sequence,
        record: record.clone(),
    };
    let sign_bytes_hash = hash(to_borsh_vec(&sign_bytes)?);
    let signature = sk.sign_digest(&sign_bytes_hash.into_slice());
    Ok(Header {
        signature: signature.into(),
        record,
    })
}
