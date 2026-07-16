use anyhow::{bail, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};

#[derive(Default)]
pub struct CnetBackend;

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://paste\.c-net\.org/[A-Za-z0-9]+").unwrap())
}

#[async_trait]
impl Backend for CnetBackend {
    fn name(&self) -> &'static str {
        "cnet"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        let mut headers = HashMap::new();
        headers.insert("Accept".into(), "text/plain".into());
        let body = upload_file(
            client,
            MultipartUpload {
                endpoint: "https://paste.c-net.org/".into(),
                file_path: item.path.clone(),
                file_name: item.name.clone(),
                file_size: item.size,
                field_name: "file".into(),
                extra_fields: HashMap::new(),
                headers,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("upload returns error: {e}"))?;
        parse_link(&String::from_utf8_lossy(&body))
    }
}

fn parse_link(body: &str) -> Result<String> {
    let resp = body.trim();
    if resp.starts_with("http://") || resp.starts_with("https://") {
        if let Some(m) = link_re().find(resp) {
            return Ok(m.as_str().to_string());
        }
        return Ok(resp.split_whitespace().next().unwrap_or(resp).to_string());
    }
    if let Some(m) = link_re().find(resp) {
        return Ok(m.as_str().to_string());
    }
    let short = if resp.len() > 180 {
        format!("{}...", &resp[..180])
    } else {
        resp.to_string()
    };
    bail!("upload error: cannot find download link in response: {short}")
}
