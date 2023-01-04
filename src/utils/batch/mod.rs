use ethers::providers::{IpcError, ProviderError};

use self::common::{BatchRequest, BatchResponse};

pub mod common;
pub mod custom_ipc;

pub struct BatchProvider<P> {
    pub inner: P,
}

impl BatchProvider<custom_ipc::Ipc> {
    pub async fn connect_ipc(path: impl AsRef<std::path::Path>) -> Result<Self, ProviderError> {
        let ipc = custom_ipc::Ipc::connect(path).await.unwrap();
        Ok(Self { inner: ipc })
    }

    pub async fn execute_batch(&self, batch: &mut BatchRequest) -> Result<BatchResponse, IpcError> {
        self.inner.execute_batch(batch).await
    }
}
