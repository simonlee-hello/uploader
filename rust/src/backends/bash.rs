use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct BashBackend;

#[async_trait]
impl Backend for BashBackend {
    fn name(&self) -> &'static str {
        "bash"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://bashupload.com/".into(),
                file_path: item.path.clone(),
                file_name: item.name.clone(),
                file_size: item.size,
                field_name: "file".into(),
                extra_fields: HashMap::new(),
                headers: HashMap::new(),
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("upload returns error: {e}"))?;
        Ok(String::from_utf8_lossy(&body).trim().to_string())
    }
}
