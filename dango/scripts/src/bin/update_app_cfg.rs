use {
    dango_testing::TestAccount,
    dango_types::{
        account::spot::QueryMsg,
        auth::Metadata,
        config::{AppAddresses, AppConfig},
    },
    grug::{Addr, Denom, JsonSerExt, MsgConfigure, NonEmpty, ResultExt, Signer, UnsignedTx},
    grug_client::Client,
    std::{collections::BTreeMap, str::FromStr, sync::LazyLock},
};

const OWNER_SEED_PHRASE: &str =  "success away current amateur choose crystal busy labor cost genius industry cement rhythm refuse whale admit meadow truck edge tiger melt flavor weapon august";

static OWNER_ADDR: LazyLock<Addr> =
    LazyLock::new(|| Addr::from_str("0xb86b2d96971c32f68241df04691479edb6a9cd3b").unwrap());

#[tokio::main]
async fn main() {
    let client = Client::connect("http://65.108.46.248:26657").unwrap();

    let cfg = AppConfig {
        dango: Denom::from_str("udg").unwrap(),
        addresses: AppAddresses {
            account_factory: Addr::from_str("0x7f3a53d1f240e043a105fb59eac2cc10496bfb92").unwrap(),
            ibc_transfer: Addr::from_str("0xfd802a93e35647c5cbd3c85e5816d1994490271e").unwrap(),
            lending: Addr::from_str("0x5981ae625871c498afda8e9a52e3abf5f5486578").unwrap(),
            oracle: Addr::from_str("0x9ec674c981c0ec87a74dd7c4e9788d21003a2f79").unwrap(),
        },
        collateral_powers: BTreeMap::default(),
    };

    let sequence: u32 = client
        .query_wasm_smart(*OWNER_ADDR, &QueryMsg::Sequence {}, None)
        .await
        .unwrap();

    let mut account =
        TestAccount::new_from_seed_phrase("owner", OWNER_SEED_PHRASE).with_address(*OWNER_ADDR);

    account.sequence = sequence;

    let msg = grug::Message::Configure(MsgConfigure {
        new_cfg: None,
        new_app_cfg: Some(cfg.to_json_value().unwrap()),
    });

    let metadata = Metadata {
        username: account.username.clone(),
        key_hash: account.key_hash,
        sequence,
    };

    let unsigned_tx = UnsignedTx {
        sender: *OWNER_ADDR,
        msgs: NonEmpty::new_unchecked(vec![msg.clone()]),
        data: metadata.clone().to_json_value().unwrap(),
    };

    let simulation_result = client.simulate(&unsigned_tx).await.should_succeed();

    let signed_tx = account
        .sign_transaction(vec![msg], "dev-3", simulation_result.gas_used + 2_000_000)
        .should_succeed();

    client.broadcast_tx(signed_tx).await.should_succeed();
}
