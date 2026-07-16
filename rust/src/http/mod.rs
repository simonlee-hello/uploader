pub mod multipart;

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

pub const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

pub fn build_client(timeout_secs: u64) -> Result<Client> {
    let mut b = Client::builder()
        .user_agent(DEFAULT_UA)
        .use_rustls_tls()
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(30));

    if timeout_secs > 0 {
        b = b.timeout(Duration::from_secs(timeout_secs));
    }

    b.build().context("build http client")
}
