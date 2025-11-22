use tokio::runtime::{Builder, Handle, Runtime};

/// Lightweight runtime wrapper to run async from sync contexts
#[derive(Debug)]
pub struct RuntimeHandler {
    runtime: Option<Runtime>,
    handle: Handle,
}

impl Default for RuntimeHandler {
    fn default() -> Self {
        match Handle::try_current() {
            Ok(handle) => Self {
                runtime: None,
                handle,
            },
            Err(_) => {
                let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
                let handle = runtime.handle().clone();
                Self {
                    runtime: Some(runtime),
                    handle,
                }
            },
        }
    }
}

impl RuntimeHandler {
    pub fn from_handle(handle: Handle) -> Self {
        Self {
            runtime: None,
            handle,
        }
    }

    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    pub fn block_on<F, R>(&self, fut: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        if self.runtime.is_some() {
            self.handle.block_on(fut)
        } else {
            tokio::task::block_in_place(|| self.handle.block_on(fut))
        }
    }
}
