use {grug_malachite::ActorsConfig, std::path::Path};

pub fn load_config(path: impl AsRef<Path>, prefix: Option<&str>) -> anyhow::Result<ActorsConfig> {
    ::config::Config::builder()
        .add_source(::config::File::from(path.as_ref()))
        .add_source(
            ::config::Environment::with_prefix(prefix.unwrap_or("MALACHITE")).separator("__"),
        )
        .build()?
        .try_deserialize()
        .map_err(Into::into)
}
