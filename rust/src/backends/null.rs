use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;

use super::{Backend, UploadItem};

pub struct NullBackend;

#[async_trait]
impl Backend for NullBackend {
    fn name(&self) -> &'static str {
        "nil"
    }

    async fn upload(&mut self, _client: &Client, _item: &UploadItem) -> Result<String> {
        bail!("0x0.st uploads disabled")
    }
}
