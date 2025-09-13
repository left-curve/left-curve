pub mod common_function;

use {
    crate::common_function::test_stream,
    grug::NonEmpty,
    indexer_disk_saver::persistence::DiskPersistence,
    pyth_lazer::{PYTH_CACHE_SAMPLES, PythClientLazerCache},
    pyth_types::{
        LeEcdsaMessage,
        constants::{
            ATOM_USD_ID_LAZER, BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER, ETH_USD_ID_LAZER,
            LAZER_ENDPOINTS_TEST, LAZER_ID_ALL,
        },
    },
};

#[tokio::test]
async fn test_lazer_stream() {
    let client =
        PythClientLazerCache::new(NonEmpty::new_unchecked(LAZER_ENDPOINTS_TEST), "lazer-token")
            .unwrap();
    test_stream(client, vec![BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER], vec![
        ETH_USD_ID_LAZER,
        ATOM_USD_ID_LAZER,
    ])
    .await;
}

// This test is used to create the cache files for Pyth Lazer.
// To update the cache files, delete the cached files in pyth/client/testdata/lazer/*.
#[ignore = "rely on network calls"]
#[test]
fn create_cache() {
    let update_ids = LAZER_ID_ALL;

    let mut client =
        PythClientLazerCache::new(NonEmpty::new_unchecked(LAZER_ENDPOINTS_TEST), "lazer-token")
            .unwrap();
    client.load_or_retrieve_data(NonEmpty::new_unchecked(update_ids.clone()));

    for id in update_ids {
        let filename = PythClientLazerCache::cache_filename(&id);

        let mut cache_file = DiskPersistence::new(filename, true);

        assert!(cache_file.exists(), "Cache file for {id} not found");

        // Load the cache file to ensure it was created correctly.
        let data = cache_file.load::<Vec<Vec<LeEcdsaMessage>>>().unwrap();

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
