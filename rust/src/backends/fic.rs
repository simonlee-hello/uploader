use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::sync::OnceLock;

use super::{Backend, UploadItem};
use crate::http::multipart::{upload_file, MultipartUpload};
use crate::util::ftp::ftp_upload;

const FTP_SERVER: &str = "ftp.1fichier.com:21";
const FTP_USER: &str = "1fichierisgood";
const FTP_PASS: &str = "1fichier";

const DOMAINS: &[&str] = &[
    "1fichier.com",
    "alterupload.com",
    "cjoint.net",
    "desfichiers.com",
    "dfichiers.com",
    "megadl.fr",
    "mesfichiers.org",
    "piecejointe.net",
    "pjointe.com",
    "tenvoi.com",
    "dl4free.com",
];

pub struct FicBackend {
    password: String,
    quiet: bool,
    use_ftp: bool,
}

impl FicBackend {
    pub fn new(password: String, quiet: bool, use_ftp: bool) -> Self {
        Self {
            password,
            quiet,
            use_ftp,
        }
    }
}

#[derive(Deserialize)]
struct UploadServer {
    url: String,
    id: String,
}

fn download_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let domains = DOMAINS
            .iter()
            .map(|d| regex::escape(d))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&format!(r"https://(?:{domains})/\?\w+")).unwrap()
    })
}

fn remove_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let domains = DOMAINS
            .iter()
            .map(|d| regex::escape(d))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&format!(r"https://(?:{domains})/remove/[\w/]+")).unwrap()
    })
}

async fn get_upload_url(client: &Client) -> Result<String> {
    let url = "https://api.1fichier.com/v1/upload/get_upload_server.cgi";
    let resp = client
        .get(url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("get upload server")?;
    let body: UploadServer = resp.json().await.context("parse upload server")?;
    Ok(format!("https://{}/upload.cgi?id={}", body.url, body.id))
}

#[async_trait]
impl Backend for FicBackend {
    fn name(&self) -> &'static str {
        "fic"
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        if self.use_ftp {
            let path = item.path.clone();
            let name = item.name.clone();
            let quiet = self.quiet;
            match tokio::task::spawn_blocking(move || -> Result<()> {
                let f = File::open(&path).with_context(|| path.display().to_string())?;
                ftp_upload(FTP_SERVER, FTP_USER, FTP_PASS, &name, f)
            })
            .await
            .context("ftp join")?
            {
                Ok(()) => return Ok("ftp ok".into()),
                Err(e) => {
                    if !quiet {
                        eprintln!("ftp: {e:#}");
                    }
                    // fall through to HTTP
                }
            }
        }

        if !self.quiet {
            eprintln!("upload Via HTTP...");
        }
        let mut last_err = None;
        for (domain_id, _) in DOMAINS.iter().enumerate() {
            let upload_url = match get_upload_url(client).await {
                Ok(u) => u,
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            };
            let mut extra = HashMap::new();
            extra.insert("mail".into(), String::new());
            extra.insert("domain".into(), domain_id.to_string());
            extra.insert("dpass".into(), self.password.clone());

            let mut headers = HashMap::new();
            headers.insert("Referer".into(), "https://1fichier.com/".into());
            headers.insert(
                "Accept".into(),
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".into(),
            );

            match upload_file(
                client,
                MultipartUpload {
                    endpoint: upload_url.trim().to_string(),
                    file_path: item.path.clone(),
                    file_name: item.name.clone(),
                    file_size: item.size,
                    field_name: "file[]".into(),
                    extra_fields: extra,
                    headers,
                },
            )
            .await
            {
                Ok(body) => {
                    let text = String::from_utf8_lossy(&body);
                    let link = download_re()
                        .find(&text)
                        .map(|m| m.as_str().to_string())
                        .ok_or_else(|| anyhow::anyhow!("no download link in response"))?;
                    if !self.quiet {
                        if let Some(m) = remove_re().find(&text) {
                            eprintln!("remove: {}", m.as_str());
                        }
                        if !self.password.is_empty() {
                            eprintln!("password: {}", self.password);
                        }
                    }
                    return Ok(link);
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }
        bail!(
            "upload returns error: {}",
            last_err.unwrap_or_else(|| anyhow::anyhow!("all domains failed"))
        )
    }
}
