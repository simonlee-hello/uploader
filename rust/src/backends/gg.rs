use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct GgBackend;

#[async_trait]
impl Backend for GgBackend {
    fn name(&self) -> &'static str {
        "gg"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://download.gg/server/upload5555.php".into(),
                file_path: item.path.clone(),
                file_name: item.name.clone(),
                file_size: item.size,
                field_name: "file[]".into(),
                extra_fields: HashMap::new(),
                headers: HashMap::new(),
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("upload returns error: {e}"))?;
        let raw = String::from_utf8_lossy(&body);
        let ref_link = format!(
            "https://download.gg/file-{}",
            raw.replace('&', "_").trim()
        );
        Ok(ref_link)
    }
}
