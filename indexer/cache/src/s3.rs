use {
    aws_config::{BehaviorVersion, Region},
    aws_credential_types::Credentials,
    aws_sdk_s3::{Client, config::Builder as S3ConfigBuilder, primitives::ByteStream},
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct S3Config {
    pub enabled: bool,
    pub path: String,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
}

impl S3Config {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn client(&self) -> anyhow::Result<Client> {
        let creds = Credentials::new(
            self.access_key.clone(),
            self.secret_key.clone(),
            None,
            None,
            "static",
        );
        let base_cfg = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(self.region.clone()))
            .credentials_provider(creds)
            .load()
            .await;

        let mut builder = S3ConfigBuilder::from(&base_cfg);

        if !self.endpoint.is_empty() {
            builder = builder.endpoint_url(self.endpoint.clone());
        }

        Ok(Client::from_conf(builder.build()))
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(key = %key)))]
pub async fn upload_file(cfg: S3Config, key: String, file_path: &PathBuf) -> anyhow::Result<()> {
    let client = cfg.client().await?;

    let body = ByteStream::from_path(file_path).await?;

    client
        .put_object()
        .bucket(cfg.bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    Ok(())
}
