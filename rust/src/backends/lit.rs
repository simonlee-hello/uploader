use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct LitBackend;

#[async_trait]
impl Backend for LitBackend {
    fn name(&self) -> &'static str {
        "lit"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let mut extra = HashMap::new();
        extra.insert("reqtype".into(), "fileupload".into());
        extra.insert("time".into(), "72h".into());
        extra.insert("u_key".into(), rand_key(16));

        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://litterbox.catbox.moe/resources/internals/api.php".into(),
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

fn rand_key(n: usize) -> String {
    use rand::Rng;
    const A: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..n).map(|_| A[rng.gen_range(0..A.len())] as char).collect()
}
