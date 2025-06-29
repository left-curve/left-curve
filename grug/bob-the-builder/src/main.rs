use {
    glob::glob,
    serde::Deserialize,
    std::{
        fs::{self, File},
        path::Path,
        process::Command,
    },
};

const TARGET_DIR: &str = "/target";
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
    // Create the artifacts directory if it doesn't exist.
    // Otherwise, empty its content.
    let path = Path::new(ARTIFACTS_DIR);
    if path.exists() {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
        } else {
            panic!("output directory `{ARTIFACTS_DIR}` exists but is not a directory");
        }
    } else {
        fs::create_dir(path).unwrap();
    }

    // Delete previously built artifacts that have been cached.
    for path in glob(&format!("{TARGET_DIR}/wasm32-unknown-unknown/release/*.wasm"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    {
        fs::remove_file(path).unwrap();
    }

    // Read the workspace root `Cargo.toml` file.
    let file = fs::read_to_string("Cargo.toml").unwrap();
    let cargo_toml = toml::from_str::<CargoToml>(&file).unwrap();

    // Find all workspace members that we're going to build.
    let mut members = cargo_toml
        .workspace
        .expect("not a cargo workspace")
        .members
        .expect("workspace does not contain any member")
        .into_iter()
        .flat_map(|member| {
            // The member can be a path to a specific folder (e.g. `grug/tester`)
            // or a whildcard (e.g. `dango/*`). For the latter case, we need to
            // expand the wildcard, and filter off results that aren't directories.
            glob(&member).unwrap().filter_map(|path| {
                if path.as_ref().unwrap().is_dir() {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .filter_map(|path| {
            // Read the member's `Cargo.toml` file.
            let file = fs::read_to_string(path.unwrap().join("Cargo.toml")).unwrap();
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
    // This is for reproducibility - we're unsure if the build output is
    // dependent on the order.
    members.sort();

    println!("contracts to build:");
    for member in &members {
        println!("- {member}");
    }

    // Build the crates.
    // To be safe, we do this synchrously, i.e. wait for one crate to finish
    // building before moving on to the next one. This mean using `.status()`
    // instead of `.spawn()`.
    // Again, this is for reproducibility. We're unsure if building in parallel
    // will affect the build output.
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
            .status()
            .unwrap();
        assert!(build.success());
    }

    // Optimize the wasm artifacts.
    for member in members {
        println!("optimizing {member}...");

        // Convert package name to snake_case.
        let member = member.replace('-', "_");

        let output = Command::new("wasm-opt")
            .arg("-Os") // execute default optimization passes, focusing on code size
            .arg(format!("{TARGET_DIR}/wasm32-unknown-unknown/release/{member}.wasm"))
            .arg("-o")
            .arg(format!("{ARTIFACTS_DIR}/{member}.wasm"))
            .status()
            .unwrap();
        assert!(output.success());
    }

    // Do checksum.
    let artifacts = glob(&format!("{ARTIFACTS_DIR}/*.wasm"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let checksums = Command::new("sha256sum")
        .args(artifacts)
        .stdout(File::create(format!("{ARTIFACTS_DIR}/checksum.txt")).unwrap())
        .status()
        .unwrap();
    assert!(checksums.success());

    println!("done 🤟");
}
