use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct TempBackend {
    last: String,
}

#[async_trait]
impl Backend for TempBackend {
    fn name(&self) -> &'static str {
        "temp"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://temp.sh/upload".into(),
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
        let s = String::from_utf8_lossy(&body).trim().to_string();
        if !(s.starts_with("http://") || s.starts_with("https://")) {
            bail!("upload error: {s}");
        }
        self.last = s.clone();
        Ok(s)
    }
}
