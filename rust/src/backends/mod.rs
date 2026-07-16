mod bash;
mod cat;
mod cnet;
mod fic;
mod gof;
mod gg;
mod lit;
mod null;
mod temp;
mod tmpf;
mod wss;

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct UploadItem {
    pub path: std::path::PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CreateOpts {
    pub single: bool,
    pub password: String,
    pub cookie: String,
    pub quiet: bool,
    pub ftp: bool,
}

#[async_trait]
pub trait Backend: Send {
    fn name(&self) -> &'static str;

    async fn init(&mut self, _client: &Client, _items: &[UploadItem]) -> Result<()> {
        Ok(())
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String>;

    async fn finish(&mut self, _client: &Client) -> Result<Option<String>> {
        Ok(None)
    }
}

pub fn create(name: &str, opts: &CreateOpts) -> Result<Box<dyn Backend>> {
    let n = name.trim().to_ascii_lowercase();
    Ok(match n.as_str() {
        "temp" | "tempsh" => Box::new(temp::TempBackend::default()),
        "tmpf" | "tempfiles" | "tmpfiles" => Box::new(tmpf::TmpfBackend::default()),
        "lit" | "litterbox" => Box::new(lit::LitBackend::default()),
        "cnet" | "paste" => Box::new(cnet::CnetBackend::default()),
        "gg" | "downloadgg" => Box::new(gg::GgBackend::default()),
        "gof" | "gofile" => Box::new(gof::GofBackend::new(opts.single, opts.quiet)),
        "fic" | "1fichier" => Box::new(fic::FicBackend::new(
            opts.password.clone(),
            opts.quiet,
            opts.ftp,
        )),
        "wss" | "wenshushu" => Box::new(wss::WssBackend::new(
            opts.password.clone(),
            opts.cookie.clone(),
            opts.quiet,
            opts.single,
        )),
        "cat" | "catbox" => Box::new(cat::CatBackend::default()),
        "bash" | "bashupload" => Box::new(bash::BashBackend::default()),
        "nil" | "null" | "0x0" => Box::new(null::NullBackend),
        _ => bail!("unknown backend {name:?}"),
    })
}

pub fn upload_name(path: &Path, encrypt: bool) -> String {
    let mut name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    if encrypt {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() {
            name.push_str(".bin");
        }
    }
    name
}
