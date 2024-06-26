use {
    anyhow::anyhow,
    grug_types::{Addr, Coin, Coins, StdResult},
    std::collections::BTreeMap,
};

const PATH_ARTIFACTS: &str = "../../../artifacts";

pub fn read_wasm_file(file_name: &str) -> anyhow::Result<Vec<u8>> {
    std::fs::read(format!("{PATH_ARTIFACTS}/{file_name}.wasm")).map_err(|_| {
        anyhow!(
            " {} not found in {}{}",
            file_name,
            std::env::current_dir().unwrap().to_str().unwrap(),
            PATH_ARTIFACTS
        )
    })
}

pub fn init_balances(balances: Vec<(&Addr, Vec<Coin>)>) -> StdResult<BTreeMap<Addr, Coins>> {
    balances
        .into_iter()
        .map(|(addr, coins)| -> StdResult<(Addr, Coins)> { Ok((addr.clone(), coins.try_into()?)) })
        .collect()
}
