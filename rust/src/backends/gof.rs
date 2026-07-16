use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct GofBackend {
    single: bool,
    quiet: bool,
    server_link: String,
    user_token: String,
    folder_id: String,
    download_link: String,
}

impl GofBackend {
    pub fn new(single: bool, quiet: bool) -> Self {
        Self {
            single,
            quiet,
            ..Default::default()
        }
    }

    async fn select_server(&mut self, client: &Client) -> Result<()> {
        if !self.quiet {
            eprint!("selecting server...");
        }
        let resp = client
            .get("https://api.gofile.io/servers")
            .send()
            .await
            .context("request servers")?;
        let body: ServersResp = resp.json().await.context("parse servers")?;
        let name = body
            .data
            .servers
            .first()
            .map(|s| s.name.trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("no gofile server"))?;
        if !self.quiet {
            eprintln!(" {name}");
        }
        self.server_link = format!("https://{name}.gofile.io/contents/uploadfile");
        Ok(())
    }
}

#[derive(Deserialize)]
struct ServersResp {
    data: ServersData,
}

#[derive(Deserialize)]
struct ServersData {
    servers: Vec<ServerInfo>,
}

#[derive(Deserialize)]
struct ServerInfo {
    name: String,
}

#[derive(Deserialize)]
struct UploadResp {
    status: Option<String>,
    data: UploadData,
}

#[derive(Deserialize)]
struct UploadData {
    #[serde(rename = "downloadPage")]
    download_page: String,
    #[serde(rename = "guestToken", default)]
    guest_token: String,
    #[serde(rename = "parentFolder", default)]
    parent_folder: String,
}

#[async_trait]
impl Backend for GofBackend {
    fn name(&self) -> &'static str {
        "gof"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        if self.server_link.is_empty() {
            self.select_server(client).await?;
        }
        let mut extra = HashMap::new();
        extra.insert("token".into(), self.user_token.clone());
        extra.insert("folderId".into(), self.folder_id.clone());

        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: self.server_link.clone(),
                file_path: item.path.clone(),
                file_name: item.name.clone(),
                file_size: item.size,
                field_name: "file".into(),
                extra_fields: extra,
                headers: HashMap::new(),
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("upload returns error: {e}"))?;

        let resp: UploadResp = serde_json::from_slice(&body).context("parse gofile response")?;
        if let Some(st) = &resp.status {
            if st != "ok" {
                bail!("upload failed: {}", String::from_utf8_lossy(&body));
            }
        }
        self.download_link = resp.data.download_page;
        if self.single {
            if self.user_token.is_empty() && !resp.data.guest_token.is_empty() {
                self.user_token = resp.data.guest_token;
            }
            if self.folder_id.is_empty() && !resp.data.parent_folder.is_empty() {
                self.folder_id = resp.data.parent_folder;
            }
            return Ok(String::new()); // deferred to finish
        }
        Ok(self.download_link.clone())
    }

    async fn finish(&mut self, _client: &Client) -> Result<Option<String>> {
        if self.single && !self.download_link.is_empty() {
            return Ok(Some(self.download_link.clone()));
        }
        Ok(None)
    }
}
