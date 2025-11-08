use {crate::traits::QueryApp, std::sync::Arc};

#[derive(Clone)]
pub struct Context {
    pub grug_app: Arc<dyn QueryApp + Send + Sync>,
}

impl Context {
    pub fn new(grug_app: Arc<dyn QueryApp + Send + Sync>) -> Self {
        Self { grug_app }
    }
}
