use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct CatBackend;

#[async_trait]
impl Backend for CatBackend {
    fn name(&self) -> &'static str {
        "cat"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let mut extra = HashMap::new();
        extra.insert("reqtype".into(), "fileupload".into());
        extra.insert("userhash".into(), String::new());
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://catbox.moe/user/api.php".into(),
                file_path: item.path.clone(),
                file_name: item.name.clone(),
                file_size: item.size,
                field_name: "fileToUpload".into(),
                extra_fields: extra,
                headers: HashMap::new(),
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("upload returns error: {e}"))?;
        Ok(String::from_utf8_lossy(&body).trim().to_string())
    }
}
