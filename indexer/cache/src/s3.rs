use {
    crate::error::{IndexerError, Result},
    aws_config::{BehaviorVersion, Region},
    aws_credential_types::Credentials,
    aws_sdk_s3::{
        Client as AwsS3Client, config::Builder as S3ConfigBuilder, error::SdkError,
        operation::head_object::HeadObjectError, primitives::ByteStream,
    },
    serde::{Deserialize, Serialize},
    std::path::Path,
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
    async fn client(&self) -> Result<AwsS3Client> {
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

        Ok(AwsS3Client::from_conf(builder.build()))
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    cfg: S3Config,
    inner: AwsS3Client,
}

impl Client {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn new(cfg: S3Config) -> Result<Self> {
        let inner = cfg.client().await?;
        Ok(Self { cfg, inner })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(key = %key)))]
    pub async fn upload_file(&self, key: String, file_path: &Path) -> Result<()> {
        let body = ByteStream::from_path(file_path)
            .await
            .map_err(|e| IndexerError::byte_stream(e.to_string()))?;

        self.inner
            .put_object()
            .bucket(&self.cfg.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| IndexerError::s3(e.to_string()))?;

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(key = %key)))]
    pub async fn delete_file(&self, key: String) -> Result<()> {
        self.inner
            .delete_object()
            .bucket(&self.cfg.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| IndexerError::s3(e.to_string()))?;

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(key = %key)))]
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let res = self
            .inner
            .head_object()
            .bucket(&self.cfg.bucket)
            .key(key)
            .send()
            .await;

        Ok(match res {
            Ok(_) => true,
            // Service error and it's specifically a NotFound => doesn't exist
            Err(SdkError::ServiceError(inner))
                if matches!(inner.err(), HeadObjectError::NotFound(_)) =>
            {
                false
            },
            // anything else is a real error
            Err(e) => return Err(IndexerError::s3(e.to_string())),
        })
    }
}
