use {
    anyhow::ensure,
    grug::{Hash256, hash},
    sha3::{Digest, Keccak256},
};

pub const TREE_DEPTH: usize = 32;

pub const MAX_LEAVES: u128 = (1 << TREE_DEPTH) - 1;

pub const ZERO_HASHES: [Hash256; TREE_DEPTH] = [
    hash!("0000000000000000000000000000000000000000000000000000000000000000"),
    hash!("ad3228b676f7d3cd4284a5443f17f1962b36e491b30a40b2405849e597ba5fb5"),
    hash!("b4c11951957c6f8f642c4af61cd6b24640fec6dc7fc607ee8206a99e92410d30"),
    hash!("21ddb9a356815c3fac1026b6dec5df3124afbadb485c9ba5a3e3398a04b7ba85"),
    hash!("e58769b32a1beaf1ea27375a44095a0d1fb664ce2dd358e7fcbfb78c26a19344"),
    hash!("0eb01ebfc9ed27500cd4dfc979272d1f0913cc9f66540d7e8005811109e1cf2d"),
    hash!("887c22bd8750d34016ac3c66b5ff102dacdd73f6b014e710b51e8022af9a1968"),
    hash!("ffd70157e48063fc33c97a050f7f640233bf646cc98d9524c6b92bcf3ab56f83"),
    hash!("9867cc5f7f196b93bae1e27e6320742445d290f2263827498b54fec539f756af"),
    hash!("cefad4e508c098b9a7e1d8feb19955fb02ba9675585078710969d3440f5054e0"),
    hash!("f9dc3e7fe016e050eff260334f18a5d4fe391d82092319f5964f2e2eb7c1c3a5"),
    hash!("f8b13a49e282f609c317a833fb8d976d11517c571d1221a265d25af778ecf892"),
    hash!("3490c6ceeb450aecdc82e28293031d10c7d73bf85e57bf041a97360aa2c5d99c"),
    hash!("c1df82d9c4b87413eae2ef048f94b4d3554cea73d92b0f7af96e0271c691e2bb"),
    hash!("5c67add7c6caf302256adedf7ab114da0acfe870d449a3a489f781d659e8becc"),
    hash!("da7bce9f4e8618b6bd2f4132ce798cdc7a60e7e1460a7299e3c6342a579626d2"),
    hash!("2733e50f526ec2fa19a22b31e8ed50f23cd1fdf94c9154ed3a7609a2f1ff981f"),
    hash!("e1d3b5c807b281e4683cc6d6315cf95b9ade8641defcb32372f1c126e398ef7a"),
    hash!("5a2dce0a8a7f68bb74560f8f71837c2c2ebbcbf7fffb42ae1896f13f7c7479a0"),
    hash!("b46a28b6f55540f89444f63de0378e3d121be09e06cc9ded1c20e65876d36aa0"),
    hash!("c65e9645644786b620e2dd2ad648ddfcbf4a7e5b1a3a4ecfe7f64667a3f0b7e2"),
    hash!("f4418588ed35a2458cffeb39b93d26f18d2ab13bdce6aee58e7b99359ec2dfd9"),
    hash!("5a9c16dc00d6ef18b7933a6f8dc65ccb55667138776f7dea101070dc8796e377"),
    hash!("4df84f40ae0c8229d0d6069e5c8f39a7c299677a09d367fc7b05e3bc380ee652"),
    hash!("cdc72595f74c7b1043d0e1ffbab734648c838dfb0527d971b602bc216c9619ef"),
    hash!("0abf5ac974a1ed57f4050aa510dd9c74f508277b39d7973bb2dfccc5eeb0618d"),
    hash!("b8cd74046ff337f0a7bf2c8e03e10f642c1886798d71806ab1e888d9e5ee87d0"),
    hash!("838c5655cb21c6cb83313b5a631175dff4963772cce9108188b34ac87c81c41e"),
    hash!("662ee4dd2dd7b2bc707961b1e646c4047669dcb6584f0d8d770daf5d7e7deb2e"),
    hash!("388ab20e2573d171a88108e79d820e98f26c0b84aa8b2f4aa4968dbb818ea322"),
    hash!("93237c50ba75ee485f4c22adf2f741400bdf8d6a9cc7df7ecae576221665d735"),
    hash!("8448818bb4ae4562849e949e17ac16e0be16688e156b5cf15e098c627c0056a9"),
];

/// Reference:
/// <https://medium.com/@josephdelong/ethereum-2-0-deposit-merkle-tree-13ec8404ca4f>
#[grug::derive(Serde, Borsh)]
pub struct IncrementalMerkleTree {
    pub branch: [Hash256; TREE_DEPTH],
    pub count: u128,
}

impl Default for IncrementalMerkleTree {
    fn default() -> Self {
        Self {
            branch: ZERO_HASHES,
            count: 0,
        }
    }
}

impl IncrementalMerkleTree {
    pub fn insert(&mut self, node: Hash256) -> anyhow::Result<()> {
        ensure!(self.count < MAX_LEAVES, "tree is full");

        self.count += 1;

        let mut node = node;
        let mut size = self.count;

        for (i, next) in self.branch.iter().enumerate() {
            if (size & 1) == 1 {
                self.branch[i] = node;
                return Ok(());
            }

            node = keccak256_two(next, node);
            size /= 2;
        }

        unreachable!();
    }

    pub fn root(&self) -> Hash256 {
        let mut current = Hash256::ZERO;

        for (i, zero) in ZERO_HASHES.iter().enumerate() {
            let ith_bit = (self.count >> i) & 1;
            let next = &self.branch[i];

            if ith_bit == 1 {
                current = keccak256_two(next, current);
            } else {
                current = keccak256_two(current, zero);
            }
        }

        current
    }
}

fn keccak256_two<A, B>(a: A, b: B) -> Hash256
where
    A: AsRef<[u8]>,
    B: AsRef<[u8]>,
{
    let mut hasher = Keccak256::new();
    hasher.update(a);
    hasher.update(b);
    Hash256::from_inner(hasher.finalize().into())
}

// ----------------------------------- tests -----------------------------------

// Adapted from:
// https://github.com/hyperlane-xyz/hyperlane-monorepo/blob/main/solidity/test/merkle.test.ts
#[cfg(test)]
mod tests {
    use {super::*, crate::eip191_hash, serde::Deserialize};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCase {
        pub test_name: String,
        pub expected_root: String,
        pub leaves: Vec<String>,
    }

    #[test]
    fn incremental_merkle_tree_insertion_works() {
        let cases = include_str!("../testdata/merkle.json");
        let cases: Vec<TestCase> = serde_json::from_str(cases).unwrap();

        for case in cases {
            let mut tree = IncrementalMerkleTree::default();

            for leaf in case.leaves {
                let leaf_hash = eip191_hash(leaf);
                tree.insert(leaf_hash).unwrap();
            }

            let root = tree.root();
            let expected_root = hex::decode(&case.expected_root[2..]).unwrap();

            assert_eq!(
                root.as_ref(),
                expected_root,
                "root hash mismatch! name: {}, expect: {}, got {}",
                case.test_name,
                hex::encode(root),
                hex::encode(&expected_root)
            );
        }
    }
}
