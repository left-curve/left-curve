mod actors;
mod app;
mod context;
mod start;

pub use start::start;

type ActorResult<T> = Result<T, ractor::ActorProcessingErr>;
