use {
    crate::error::Error,
    config::{Config, Environment, File},
    std::path::PathBuf,
};

#[allow(dead_code)]
pub struct ConfigParser {}

impl ConfigParser {
    #[allow(dead_code)]
    pub fn parse<D>(path: PathBuf) -> Result<D, Error>
    where
        D: serde::de::DeserializeOwned,
    {
        let env_override = Environment::default().separator("__");

        let config = Config::builder()
            .add_source(File::from(path))
            .add_source(env_override)
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, assertor::*};

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
        std::env::set_var("INDEXER_HTTPD__TENDERMINT_ENDPOINT", "BAR");

        let config: TestSettings = ConfigParser::parse(PathBuf::from("fixtures/config_test1.toml"))
            .expect("Failed to parse file");

        assert_that!(config.tendermint_endpoint.as_str()).is_equal_to("http://localhost:26657");
        assert_that!(config.indexer_httpd.tendermint_endpoint.as_str()).is_equal_to("BAR");
    }
}
