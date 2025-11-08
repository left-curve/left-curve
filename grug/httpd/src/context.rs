use {crate::traits::QueryApp, std::sync::Arc, tokio::sync::Mutex};

#[derive(Clone)]
pub struct Context {
    pub grug_app: Arc<Mutex<dyn QueryApp + Send>>,
}

impl Context {
    pub fn new(grug_app: Arc<Mutex<dyn QueryApp + Send>>) -> Self {
        Self { grug_app }
    }
}
