use {grug::StdError, sea_orm::sqlx, std::io};

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    #[backtrace(new)]
    Io(io::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SeaOrm(sea_orm::DbErr),

    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    #[backtrace(new)]
    StdError(StdError),

    #[error(transparent)]
    #[backtrace(new)]
    PrometheusBuilder(metrics_exporter_prometheus::BuildError),

    #[error("anyhow error: {0}")]
    Anyhow(anyhow::Error),
}
