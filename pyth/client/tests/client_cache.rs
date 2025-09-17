mod common_function;

use {
    common_function::{test_latest_vaas, test_stream},
    grug::{Binary, NonEmpty},
    indexer_disk_saver::persistence::DiskPersistence,
    pyth_client::{PYTH_CACHE_SAMPLES, PythClientCoreCache, PythClientTrait},
    pyth_types::constants::{
        ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_IDS_ALL, PYTH_URL,
    },
};

#[test]
fn latest_vaas_cache() {
    let pyth_client = PythClientCoreCache::new(PYTH_URL).unwrap();
    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[tokio::test]
async fn test_sse_stream_cache() {
    let client = PythClientCoreCache::new(PYTH_URL).unwrap();
    test_stream(client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ])
    .await;
}

// This test is used to create the cache files for the Pyth.
// To update the cache files, delete the cached files in pyth/client/testdata/*.
#[ignore = "rely on network calls"]
#[test]
fn create_cache() {
    let update_ids = PYTH_IDS_ALL;

    let client = PythClientCoreCache::new(PYTH_URL).unwrap();
    client
        .get_latest_price_update(NonEmpty::new_unchecked(update_ids))
        .unwrap();

    for id in update_ids {
        let filename = PythClientCoreCache::cache_filename(&id.to_string());

        let mut cache_file = DiskPersistence::new(filename, true);

        assert!(cache_file.exists(), "Cache file for {id} not found");

        // Load the cache file to ensure it was created correctly.
        let data = cache_file.load::<Vec<Vec<Binary>>>().unwrap();

        assert_eq!(
            data.len(),
            PYTH_CACHE_SAMPLES,
            "Cache file for {} does not contain the expected number of samples; found {}, expected {}",
            id,
            data.len(),
            PYTH_CACHE_SAMPLES
        );
    }
}
