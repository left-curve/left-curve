use {
    config::{Config, Environment, File},
    serde::de::DeserializeOwned,
    std::path::Path,
};

pub fn parse_config<P, D>(path: P) -> Result<D, config::ConfigError>
where
    P: AsRef<Path>,
    D: DeserializeOwned,
{
    Config::builder()
        .add_source(File::from(path.as_ref()))
        .add_source(Environment::default().separator("__"))
        .build()?
        .try_deserialize()
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, assertor::*, std::path::PathBuf};

    #[derive(Debug, serde::Deserialize)]
    struct TestSettings {
        tendermint_endpoint: String,
        indexer_httpd: IndexerHttpd,
    }

    #[derive(Debug, serde::Deserialize)]
    struct IndexerHttpd {
        tendermint_endpoint: String,
    }

    #[test]
    fn test_parse_config_file() {
        unsafe {
            std::env::set_var("INDEXER_HTTPD__TENDERMINT_ENDPOINT", "BAR");
        }

        let config: TestSettings = parse_config(PathBuf::from("testdata/config_test1.toml"))
            .expect("Failed to parse file");

        assert_that!(config.tendermint_endpoint.as_str()).is_equal_to("http://localhost:26657");
        assert_that!(config.indexer_httpd.tendermint_endpoint.as_str()).is_equal_to("BAR");
    }
}
