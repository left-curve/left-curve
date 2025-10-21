//! Compare two Grug databases. Useful if we get an apphash mismatch error, and
//! want to check whether the DBs actually differ.

use {
    grug::{Order, Record, Storage},
    grug_app::Db,
    grug_db_disk_lite::DiskDbLite,
    std::{cmp::Ordering, path::PathBuf},
};

fn find_diffs<'a>(
    mut iter1: Box<dyn Iterator<Item = Record> + 'a>,
    mut iter2: Box<dyn Iterator<Item = Record> + 'a>,
) {
    let mut item1 = iter1.next();
    let mut item2 = iter2.next();
    let mut count = 0;

    while item1.is_some() || item2.is_some() {
        match (item1, item2) {
            (Some((k1, v1)), Some((k2, v2))) => match k1.cmp(&k2) {
                Ordering::Less => {
                    println!(
                        "diff found! key present in db1, but missing in db2: {}",
                        hex::encode(k1)
                    );

                    break;
                },
                Ordering::Greater => {
                    println!(
                        "diff found! key missing in db1, but present in db2: {}",
                        hex::encode(k2)
                    );

                    break;
                },
                Ordering::Equal => {
                    if v1 != v2 {
                        println!(
                            "diff found! key present in both dbs, but values mismatch! k: {}, v1: {}, v2: {}",
                            hex::encode(k1),
                            hex::encode(v1),
                            hex::encode(v2)
                        );

                        break;
                    }

                    item1 = iter1.next();
                    item2 = iter2.next();
                    count += 1;

                    if count % 100 == 0 {
                        println!("records checked: {count}");
                    }
                },
            },
            (None, Some((k2, _))) => {
                println!(
                    "diff found! key missing in db1, but present in db2: {}",
                    hex::encode(k2)
                );

                break;
            },
            (Some((k1, _)), None) => {
                println!(
                    "diff found! key present in db1, but missing in db2: {}",
                    hex::encode(k1)
                );

                break;
            },
            (None, None) => break,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let db1 = DiskDbLite::open::<_, Vec<u8>>(cwd.join("data_ovh1"), None)?;
    let storage1 = db1.state_storage(None)?;
    let iter1 = storage1.scan(None, None, Order::Ascending);

    let db2 = DiskDbLite::open::<_, Vec<u8>>(cwd.join("data_ovh2"), None)?;
    let storage2 = db2.state_storage(None)?;
    let iter2 = storage2.scan(None, None, Order::Ascending);

    find_diffs(iter1, iter2);

    Ok(())
}
