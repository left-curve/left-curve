mod shared;

use {
    grug_crypto::{keccak256, secp256k1_verify, secp256r1_verify, sha2_256},
    serde::Deserialize,
    shared::{read_file, validate_recover_secp256k1, validate_recover_secp256r1},
};

// -------------------------------- file struct --------------------------------

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub num_tests: usize,
    pub tests: Vec<Test>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub tc_id: i64,
    #[serde(deserialize_with = "hex::deserialize")]
    pub public_key_uncompressed: Vec<u8>,
    #[serde(deserialize_with = "hex::deserialize")]
    pub public_key_compressed: Vec<u8>,
    #[serde(deserialize_with = "hex::deserialize")]
    pub msg: Vec<u8>,
    pub sig: Sig,
    pub comment: String,
    pub valid: bool,
    pub flags: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sig {
    #[serde(deserialize_with = "hex::deserialize")]
    pub r: Vec<u8>,
    #[serde(deserialize_with = "hex::deserialize")]
    pub s: Vec<u8>,
    pub id: u8,
}

pub fn combine_signature(sig: &Sig) -> Vec<u8> {
    // the test data contains values with leading zeroes, which we need to ignore
    let first_non_zero = sig.r.iter().position(|&v| v != 0).unwrap_or_default();
    let r = &sig.r[first_non_zero..];
    let first_non_zero = sig.s.iter().position(|&v| v != 0).unwrap_or_default();
    let s = &sig.s[first_non_zero..];

    // at least one of the tests has an s that is 33 bytes long
    let r_len = r.len().max(32);
    let s_len = s.len().max(32);

    // the test data also contains values with less than 32 bytes, so we need to pad them with zeroes
    let mut signature = vec![0; r_len + s_len];
    let (r_part, s_part) = signature.split_at_mut(r_len);
    r_part[r_len - r.len()..].copy_from_slice(r);
    s_part[s_len - s.len()..].copy_from_slice(s);

    signature
}

// ------------------------------ test definition ------------------------------

macro_rules! rootberg_test {
    (
        $test_name:ident,
        $file_name:expr,
        $hash_fn:ident,
        $verify_fn:ident,
        $recover_fn:ident,
        $len:expr,
        $key_type:ident,
        $compressed:expr
    ) => {
        #[test]
        fn $test_name() {
            let File { num_tests, tests } = read_file($file_name);
            assert_eq!(num_tests, tests.len(), "Invalid number of tests");

            for test in tests {
                assert_eq!(test.$key_type.len(), $len);

                // eprintln!("Test case ID: {}", test.tc_id);

                let message_hash = $hash_fn(&test.msg);
                let signature = combine_signature(&test.sig);

                match $verify_fn(&message_hash, &signature, &test.$key_type) {
                    Ok(_) => {
                        assert!(test.valid);
                    },
                    Err(e) => {
                        assert!(!test.valid, "expected valid signature, got {:?}", e);
                    },
                }

                if test.valid {
                    $recover_fn(
                        &message_hash,
                        &signature,
                        &test.$key_type,
                        [0, 1],
                        $compressed,
                    );
                }
            }
        }
    };
    (K1 => $test_name:ident, $file_name:expr, $hash_fn:ident,compressed) => {
        rootberg_test!(
            $test_name,
            $file_name,
            $hash_fn,
            secp256k1_verify,
            validate_recover_secp256k1,
            33,
            public_key_compressed,
            true
        );
    };
    (K1 => $test_name:ident, $file_name:expr, $hash_fn:ident,uncompressed) => {
        rootberg_test!(
            $test_name,
            $file_name,
            $hash_fn,
            secp256k1_verify,
            validate_recover_secp256k1,
            65,
            public_key_uncompressed,
            false
        );
    };
    (R1 => $test_name:ident, $file_name:expr, $hash_fn:ident,compressed) => {
        rootberg_test!(
            $test_name,
            $file_name,
            $hash_fn,
            secp256r1_verify,
            validate_recover_secp256r1,
            33,
            public_key_compressed,
            true
        );
    };
    (R1 => $test_name:ident, $file_name:expr, $hash_fn:ident,uncompressed) => {
        rootberg_test!(
            $test_name,
            $file_name,
            $hash_fn,
            secp256r1_verify,
            validate_recover_secp256r1,
            65,
            public_key_uncompressed,
            false
        );
    };
}

// ------------------------------ secp256k1 tests ------------------------------

const SECP256K1_SHA256: &str = "./testdata/rootberg/ecdsa_secp256k1_sha_256_raw.json";
const SECP256K1_KECCAK256: &str = "./testdata/rootberg/ecdsa_secp256k1_keccak256_raw.json";

rootberg_test!(K1 =>
    ecdsa_secp256k1_sha256_compressed,
    SECP256K1_SHA256,
    sha2_256,
    compressed
);

rootberg_test!(K1 =>
    ecdsa_secp256k1_sha256_uncompressed,
    SECP256K1_SHA256,
    sha2_256,
    uncompressed
);

rootberg_test!(K1 =>
    ecdsa_secp256k1_keccak256_compressed,
    SECP256K1_KECCAK256,
    keccak256,
    compressed
);

rootberg_test!(K1 =>
    ecdsa_secp256k1_keccak256_uncompressed,
    SECP256K1_KECCAK256,
    keccak256,
    uncompressed
);

// ------------------------------ secp256r1 tests ------------------------------

const SECP256R1_SHA256: &str = "./testdata/rootberg/ecdsa_secp256r1_sha_256_raw.json";
const SECP256R1_KECCAK256: &str = "./testdata/rootberg/ecdsa_secp256r1_keccak256_raw.json";

rootberg_test!(R1 =>
    ecdsa_secp256r1_sha256_compressed,
    SECP256R1_SHA256,
    sha2_256,
    compressed
);

rootberg_test!(R1 =>
    ecdsa_secp256r1_sha256_uncompressed,
    SECP256R1_SHA256,
    sha2_256,
    uncompressed
);

rootberg_test!(R1 =>
    ecdsa_secp256r1_keccak256_compressed,
    SECP256R1_KECCAK256,
    keccak256,
    compressed
);

rootberg_test!(R1 =>
    ecdsa_secp256r1_keccak256_uncompressed,
    SECP256R1_KECCAK256,
    keccak256,
    uncompressed
);
