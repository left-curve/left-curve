use {
    clap::Parser,
    dango_types::{
        account_factory::{self},
        auth::Key,
        config::AppConfig,
    },
    grug::{
        __private::serde_json::json, ByteArray, EncodedBytes, HashExt, HexByteArray, Inner,
        JsonSerExt, QueryClientExt,
    },
    indexer_client::HttpClient,
    std::str::FromStr,
};

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    pk: String,

    #[arg(long, default_value = "https://api-mainnet.dango.zone")]
    url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let client = HttpClient::new(&cli.url)?;
    let cfg: AppConfig = client.query_app_config(None).await?;

    let pk = HexByteArray::<33>::from_str(&cli.pk)
        .map(Inner::into_inner)
        .or_else(|_| ByteArray::from_str(&cli.pk).map(Inner::into_inner))?;

    let key_hash = pk.hash256();

    let msg = account_factory::ExecuteMsg::UpdateKey {
        key_hash,
        key: grug::Op::Insert(Key::Secp256k1(EncodedBytes::from_inner(pk))),
    };

    let msg = json!({
        "contract": cfg.addresses.account_factory,
        "msg": msg,
        "funds": {}
    });

    println!("{}", msg.to_json_string_pretty()?);

    Ok(())
}
