use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct TmpfBackend;

#[derive(Deserialize)]
struct Resp {
    status: String,
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    url: String,
}

#[async_trait]
impl Backend for TmpfBackend {
    fn name(&self) -> &'static str {
        "tmpf"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://tmpfiles.org/api/v1/upload".into(),
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

        let resp: Resp = serde_json::from_slice(&body)
            .map_err(|e| anyhow::anyhow!("unmarshal returns error: {e}"))?;
        if resp.status != "success" {
            bail!("upload returns error: {}", String::from_utf8_lossy(&body));
        }
        Ok(resp.data.url)
    }
}
