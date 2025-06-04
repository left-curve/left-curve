mod actors;
mod app;
mod context;
mod start;
mod types;

pub use start::start;

type ActorResult<T> = Result<T, ractor::ActorProcessingErr>;
