use {
    crate::{ActorResult, context::Context},
    ractor::{Actor, async_trait},
};

pub type HostRef = malachitebft_engine::host::HostRef<Context>;
pub type HostMsg = malachitebft_engine::host::HostMsg<Context>;

pub struct Host;

#[async_trait]
impl Actor for Host {
    type Arguments = ();
    type Msg = HostMsg;
    type State = ();

    async fn pre_start(&self, myself: HostRef, _args: Self::Arguments) -> ActorResult<Self::State> {
        Ok(())
    }
}
