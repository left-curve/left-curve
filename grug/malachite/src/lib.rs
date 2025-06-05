mod actors;
mod app;
mod config;
mod context;
mod macros;
mod spawn;
mod start;
mod types;

pub use start::start;

type ActorResult<T> = Result<T, ractor::ActorProcessingErr>;
