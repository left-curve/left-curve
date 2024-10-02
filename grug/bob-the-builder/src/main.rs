use {
    serde::Deserialize,
    std::{fs, process::Command},
};

const TARGET_DIR: &str = "target";
const ARTIFACTS_DIR: &str = "artifacts";

#[derive(Deserialize, Debug)]
struct CargoToml {
    workspace: Option<Workspace>,
    package: Option<Package>,
    lib: Option<Lib>,
}

#[derive(Deserialize, Debug)]
struct Workspace {
    members: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Lib {
    #[serde(rename = "crate-type")] // TOML uses kebab-case, so rename.
    crate_type: Option<Vec<String>>,
}

#[rustfmt::skip]
fn main() {
    // Assume we are currently at the root of a Cargo workspace.
    // Read the workspace root `Cargo.toml` file.
    let file = fs::read_to_string("Cargo.toml").unwrap();
    let cargo_toml = toml::from_str::<CargoToml>(&file);

    // Find all workspace members that we're going to build.
    let mut members = cargo_toml
        .unwrap()
        .workspace
        .expect("not a cargo workspace")
        .members
        .expect("workspace does not contain any member")
        .into_iter()
        .filter_map(|path| {
            let file = fs::read_to_string(format!("{path}/Cargo.toml")).unwrap();
            let cargo_toml = toml::from_str::<CargoToml>(&file).unwrap();

            // We only build the crate if it is a cdylib.
            // If it's a cdylib, return the package name.
            match cargo_toml {
                CargoToml {
                    package: Some(Package {
                        name: Some(name),
                    }),
                    lib: Some(Lib {
                        crate_type: Some(types),
                    }),
                    ..
                } if types.contains(&"cdylib".to_string()) => Some(name),
                _ => None,
            }
        })
        .collect::<Vec<_>>();

    // Build the crates in alphabetical order.
    members.sort();

    println!("contracts to build:");
    for member in &members {
        println!("- {}", member);
    }

    // Build the crates.
    for member in &members {
        let build = Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .arg("build")
            .arg(format!("--package={member}"))
            .arg("--lib")
            .arg("--locked")
            .arg("--release")
            .arg("--target=wasm32-unknown-unknown")
            .arg(format!("--target-dir={TARGET_DIR}"))
            .current_dir(fs::canonicalize(".").unwrap())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        assert!(build.success());
    }

    // Optimize the crates.
    for member in members {
        println!("optimizing {member}...");

        // Convert package name to snake_case.
        let member = member.replace('-', "_");

        let optimize = Command::new("wasm-opt")
            .arg("-Os") // execute default optimization passes, focusing on code size
            .arg(format!("{TARGET_DIR}/wasm32-unknown-unknown/release/{member}.wasm"))
            .arg("-o")
            .arg(format!("{ARTIFACTS_DIR}/{member}.wasm"))
            .current_dir(fs::canonicalize(".").unwrap())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        assert!(optimize.success());
    }
}
