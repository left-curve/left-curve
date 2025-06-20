mod actors;
mod app;
mod codec;
mod config;
mod context;
mod macros;
mod spawn;
mod start;
mod types;

pub use {actors::*, app::*, config::*, spawn::*, start::start, types::*};

type ActorResult<T> = Result<T, ractor::ActorProcessingErr>;
