use anyhow::{bail, Context, Result};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::http::DEFAULT_UA;

pub struct MultipartUpload {
    pub endpoint: String,
    pub file_path: std::path::PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub field_name: String,
    pub extra_fields: HashMap<String, String>,
    pub headers: HashMap<String, String>,
}

pub async fn upload_file(client: &Client, cfg: MultipartUpload) -> Result<Vec<u8>> {
    let file = File::open(&cfg.file_path)
        .await
        .with_context(|| format!("open {}", cfg.file_path.display()))?;
    let stream = ReaderStream::new(file);
    let body = reqwest::Body::wrap_stream(stream);

    let mut part = Part::stream_with_length(body, cfg.file_size)
        .file_name(cfg.file_name.clone())
        .mime_str("application/octet-stream")?;

    // Some servers are picky; keep default.

    let mut form = Form::new();
    for (k, v) in &cfg.extra_fields {
        form = form.text(k.clone(), v.clone());
    }
    form = form.part(cfg.field_name, part);

    let mut req = client
        .post(&cfg.endpoint)
        .header("User-Agent", DEFAULT_UA)
        .header("Accept", "*/*");

    for (k, v) in &cfg.headers {
        req = req.header(k.as_str(), v.as_str());
    }

    let resp = req.multipart(form).send().await.context("http upload")?;
    let status = resp.status();
    let body = resp.bytes().await.context("read response")?;

    if status.as_u16() >= 400 {
        let msg = String::from_utf8_lossy(&body);
        let msg = msg.trim();
        if msg.eq_ignore_ascii_case("Blacklisted")
            || msg.to_ascii_lowercase().contains("blacklisted")
        {
            bail!("http {}: ip blacklisted", status.as_u16());
        }
        let short = if msg.len() > 120 {
            format!("{}...", &msg[..120])
        } else {
            msg.to_string()
        };
        bail!("http {}: {short}", status.as_u16());
    }
    Ok(body.to_vec())
}

pub async fn upload_bytes_field(
    client: &Client,
    endpoint: &str,
    field: &str,
    file_name: &str,
    data: Vec<u8>,
    extra_fields: HashMap<String, String>,
) -> Result<Vec<u8>> {
    let size = data.len() as u64;
    let path = write_temp(file_name, &data)?;
    let mut cfg = MultipartUpload {
        endpoint: endpoint.to_string(),
        file_path: path.clone(),
        file_name: file_name.to_string(),
        file_size: size,
        field_name: field.to_string(),
        extra_fields,
        headers: HashMap::new(),
    };
    let result = upload_file(client, cfg).await;
    let _ = tokio::fs::remove_file(&path).await;
    result
}

fn write_temp(name: &str, data: &[u8]) -> Result<std::path::PathBuf> {
    let tmp = tempfile::Builder::new()
        .prefix("up-")
        .suffix(&format!("-{name}"))
        .tempfile()?;
    std::io::Write::write_all(&mut std::io::BufWriter::new(tmp.as_file()), data)?;
    let (f, path) = tmp.keep()?;
    drop(f);
    Ok(path)
}

#[allow(dead_code)]
pub fn path_exists(p: &Path) -> bool {
    p.exists()
}
