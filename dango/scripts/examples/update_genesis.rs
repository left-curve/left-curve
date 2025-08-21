use {
    cargo_metadata::MetadataCommand,
    dango_genesis::{GenesisCodes, GenesisOption, build_genesis},
    dango_testing::Preset,
    grug::{Inner, Json, JsonDeExt, JsonSerExt},
    grug_vm_rust::RustVm,
    std::{env, fs},
};

// use justfile to run this script for all genesis.json files
// `just update-genesis`
fn main() {
    let (genesis_state, ..) =
        build_genesis(RustVm::genesis_codes(), GenesisOption::preset_test()).unwrap();

    let genesis_state = genesis_state.to_json_value().unwrap().into_inner();

    let root = MetadataCommand::new()
        .exec()
        .unwrap()
        .workspace_root
        .into_std_path_buf();

    // skip the first argument, which is the script name
    for location in env::args().skip(1) {
        let mut file_path = root.clone();
        file_path.push(location);

        let file_name = file_path.file_name().unwrap().to_str().unwrap();

        assert_eq!(
            file_name, "genesis.json",
            "File name must be genesis.json, found: {}",
            file_name
        );

        let mut file = fs::read(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read file: {file_path:?}"))
            .deserialize_json::<Json>()
            .unwrap();

        file.as_object_mut()
            .expect("Failed to get object")
            .insert("app_state".into(), genesis_state.clone());

        fs::write(file_path, file.to_json_string_pretty().unwrap()).unwrap();
    }
}
