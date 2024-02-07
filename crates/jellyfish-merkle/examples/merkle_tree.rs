use {
    cw_jellyfish_merkle::MerkleTree,
    cw_std::{Batch, MockStorage, Op},
};

fn main() -> anyhow::Result<()> {
    let mut store = MockStorage::new();
    let tree = MerkleTree::default();

    // hash("foo")
    // = 0x2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae
    // = 0b0010110000100110101101000110101101101000111111111100011010001111111110011001101101000101001111000001110100110000010000010011010000010011010000100010110101110000011001001000001110111111101000001111100110001010010111101000100001100010011001101110011110101110
    // hash("fuzz")
    // = 0x93850b707585e404e4951a3ddc1f05a34b3d4f5fc081d616f46d8a2e8f1c8e68
    // = 0b1001001110000101000010110111000001110101100001011110010000000100111001001001010100011010001111011101110000011111000001011010001101001011001111010100111101011111110000001000000111010110000101101111010001101101100010100010111010001111000111001000111001101000
    let batch = Batch::from([
        (b"foo".to_vec(), Op::Put(b"bar".to_vec())),
        (b"fuzz".to_vec(), Op::Put(b"buzz".to_vec())),
    ]);
    tree.apply(&mut store, batch)?;

    let version = tree.lateset_version(&store)?;
    println!("version = {version}");

    let root_hash = tree.root_hash(&store, None)?;
    println!("root_hash = {root_hash:?}");

    for (node_key, node) in tree.nodes(&store)? {
        println!("node_key = {node_key:?}, node = {node:?}");
    }

    for (orphaned_since_version, node_key) in tree.orphans(&store)? {
        println!("orphaned_since_version = {orphaned_since_version:?}, node_key = {node_key:?}");
    }

    Ok(())
}
