//! Write the genesis state used for tests to the CometBFT genesis file. Can be
//! used to spin up an actual network (e.g. using LocalDango) with an identical
//! environment as the tests.

use {
    dango_genesis::{GenesisOption, build_genesis},
    dango_testing::{
        GenesisCodes, Preset,
        constants::{MOCK_CHAIN_ID, MOCK_GENESIS_TIMESTAMP},
    },
    grug::{Inner, Json, JsonDeExt, JsonSerExt},
    grug_vm_hybrid::HybridVm,
    home::home_dir,
    std::fs,
};

fn main() {
    let (genesis_state, contracts, addresses) =
        build_genesis(HybridVm::genesis_codes(), GenesisOption::preset_test()).unwrap();

    println!(
        "genesis_state = {}",
        genesis_state.to_json_string_pretty().unwrap()
    );
    println!(
        "\ncontracts = {}",
        contracts.to_json_string_pretty().unwrap()
    );
    println!(
        "\naddresses = {}\n",
        addresses.to_json_string_pretty().unwrap()
    );

    let cometbft_genesis_path = home_dir().unwrap().join(".cometbft/config/genesis.json");

    let mut cometbft_genesis = fs::read(&cometbft_genesis_path)
        .unwrap()
        .deserialize_json::<Json>()
        .unwrap();

    let map = cometbft_genesis.as_object_mut().unwrap();
    map.insert("chain_id".into(), MOCK_CHAIN_ID.into());
    map.insert(
        "genesis_time".into(),
        MOCK_GENESIS_TIMESTAMP.to_rfc3339_string().into(),
    );
    map.insert(
        "app_state".into(),
        genesis_state.to_json_value().unwrap().into_inner(),
    );

    fs::write(
        cometbft_genesis_path,
        cometbft_genesis.to_json_string_pretty().unwrap(),
    )
    .unwrap();
}
