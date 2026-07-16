use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use tokio::time::sleep;

use super::{Backend, UploadItem};
use crate::crypto::wss_sign;

const ANONYMOUS: &str = "https://www.wenshushu.cn/ap/login/anonymous";
const ADD_SEND: &str = "https://www.wenshushu.cn/ap/task/addsend";
const GET_UP_ID: &str = "https://www.wenshushu.cn/ap/uploadv2/getupid";
const GET_UP_URL: &str = "https://www.wenshushu.cn/ap/uploadv2/psurl";
const COMPLETE: &str = "https://www.wenshushu.cn/ap/uploadv2/complete";
const PROCESS: &str = "https://www.wenshushu.cn/ap/ufile/getprocess";
const FINISH: &str = "https://www.wenshushu.cn/ap/task/copysend";
const TIME_TOKEN: &str = "https://www.wenshushu.cn/ag/time";

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
/// Server rejects generic en-US; match PC web client language list.
const ACCEPT_LANG: &str = "zh-CN, zh-Hans-CN;q=0.9";

pub struct WssBackend {
    password: String,
    token: String,
    quiet: bool,
    single: bool,
    block_size: usize,
    conf: Option<SendConfig>,
    inited: bool,
}

impl WssBackend {
    pub fn new(password: String, token: String, quiet: bool, single: bool) -> Self {
        Self {
            password,
            token,
            quiet,
            single,
            block_size: 1_048_576,
            conf: None,
            inited: false,
        }
    }

    async fn ensure_config(
        &mut self,
        client: &Client,
        total_size: u64,
        total_count: usize,
    ) -> Result<()> {
        if self.single && self.inited {
            return Ok(());
        }
        let conf = get_send_config(
            client,
            &self.token,
            &self.password,
            total_size,
            total_count,
        )
        .await?;
        self.token = conf.token.clone();
        self.conf = Some(conf);
        self.inited = true;
        Ok(())
    }

    async fn do_chunks(&self, client: &Client, item: &UploadItem, conf: &SendConfig) -> Result<()> {
        let mut block = self.block_size;
        if item.size / block as u64 > 10_000 {
            block = (item.size / 10_000) as usize;
            if !self.quiet {
                eprintln!("blocksize too small, set to {block}");
            }
        }

        let mut file = File::open(&item.path).with_context(|| item.path.display().to_string())?;
        let mut part: u64 = 0;
        loop {
            let mut buf = vec![0u8; block];
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            part += 1;
            buf.truncate(n);
            put_part(client, conf, &item.name, part, block, &buf).await?;
        }
        finish_parts(client, conf, &item.name).await
    }

    async fn complete_link(&self, client: &Client, conf: &SendConfig) -> Result<String> {
        complete_upload(client, conf, self.quiet).await
    }
}

#[async_trait]
impl Backend for WssBackend {
    fn name(&self) -> &'static str {
        "wss"
    }

    async fn init(&mut self, client: &Client, items: &[UploadItem]) -> Result<()> {
        if self.single {
            let total: u64 = items.iter().map(|i| i.size).sum();
            self.ensure_config(client, total, items.len()).await?;
        }
        Ok(())
    }

    async fn upload(&mut self, client: &Client, item: &UploadItem) -> Result<String> {
        if !self.single {
            self.inited = false;
            self.ensure_config(client, item.size, 1).await?;
        } else if !self.inited {
            bail!("wss single mode: init required");
        }
        let conf = self
            .conf
            .clone()
            .ok_or_else(|| anyhow::anyhow!("wss config missing"))?;
        self.do_chunks(client, item, &conf).await?;
        if self.single {
            return Ok(String::new());
        }
        self.complete_link(client, &conf).await
    }

    async fn finish(&mut self, client: &Client) -> Result<Option<String>> {
        if !self.single {
            return Ok(None);
        }
        let Some(conf) = self.conf.clone() else {
            return Ok(None);
        };
        let link = self.complete_link(client, &conf).await?;
        Ok(Some(link))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SendConfig {
    #[serde(default)]
    bid: String,
    #[serde(default)]
    tid: String,
    #[serde(rename = "ufileid", default)]
    ufileid: String,
    #[serde(rename = "upId", default)]
    upload_id: String,
    #[serde(default)]
    token: String,
    #[serde(rename = "mgr_url", default)]
    manage_url: String,
    #[serde(rename = "public_url", default)]
    public_url: String,
    #[serde(default)]
    #[allow(dead_code)]
    url: String,
    #[serde(rename = "rst", default)]
    #[allow(dead_code)]
    r: String,
}

#[derive(Deserialize)]
struct ApiResp {
    code: i32,
    message: String,
    #[serde(default)]
    data: serde_json::Value,
}

fn wss_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ORIGIN, REFERER, USER_AGENT};
    let mut h = HeaderMap::new();
    // Keep headers minimal — extra fields like `authority` trigger user-agent error:-3.
    h.insert("Prod", HeaderValue::from_static("com.wenshushu.web.pc"));
    h.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    h.insert(REFERER, HeaderValue::from_static("https://www.wenshushu.cn/"));
    h.insert(
        "Accept-Language",
        HeaderValue::from_static(ACCEPT_LANG),
    );
    h.insert(USER_AGENT, HeaderValue::from_static(UA));
    h.insert(ORIGIN, HeaderValue::from_static("https://www.wenshushu.cn"));
    h
}

/// addsend JSON body. serde_json Map is BTreeMap (sorted keys) unless preserve_order —
/// sorted keys are what the live API accepts with current verify.js signing.
fn wss_addsend_body(password: &str, total_size: u64, total_count: usize) -> String {
    let mut m = serde_json::Map::new();
    m.insert("downPreCountLimit".into(), json!(0));
    m.insert("expire".into(), json!("1"));
    m.insert("fileDisplay".into(), json!(0));
    m.insert("file_count".into(), json!(total_count));
    m.insert("file_size".into(), json!(total_size));
    m.insert("isextension".into(), json!(false));
    m.insert("notDownload".into(), json!(false));
    m.insert("notPreview".into(), json!(false));
    m.insert("notSaveTo".into(), json!(false));
    m.insert("pwd".into(), json!(password));
    m.insert("recvs".into(), json!(["social", "public"]));
    m.insert("remark".into(), json!(""));
    m.insert("sender".into(), json!(""));
    m.insert("task_traffic_limit".into(), json!(""));
    m.insert("trafficStatus".into(), json!(0));
    serde_json::Value::Object(m).to_string()
}

async fn api_post(
    client: &Client,
    url: &str,
    body: &str,
    token: &str,
    req_time: Option<&str>,
    a_code: Option<&str>,
) -> Result<ApiResp> {
    let mut last = None;
    for _ in 0..4 {
        let mut req = client
            .post(url)
            .headers(wss_headers())
            .header("X-TOKEN", token)
            .header("Content-Type", "application/json")
            .body(body.to_string());
        if let Some(t) = req_time {
            req = req.header("Req-Time", t);
        }
        if let Some(c) = a_code {
            // Header name is case-sensitive on wenshushu edge ("A-code").
            req = req.header("A-code", c);
        }
        match req.send().await {
            Ok(resp) => {
                let bytes = resp.bytes().await.unwrap_or_default();
                match serde_json::from_slice::<ApiResp>(&bytes) {
                    Ok(parsed) if parsed.message == "success" && parsed.code == 0 => {
                        return Ok(parsed);
                    }
                    Ok(parsed) => {
                        last = Some(anyhow::anyhow!(
                            "api {url}: {} ({})",
                            parsed.message,
                            String::from_utf8_lossy(&bytes)
                        ));
                    }
                    Err(e) => {
                        last = Some(anyhow::anyhow!(
                            "parse {url}: {e}; body={}",
                            String::from_utf8_lossy(&bytes)
                        ));
                    }
                }
            }
            Err(e) => last = Some(e.into()),
        }
        sleep(Duration::from_millis(400)).await;
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("request {url} failed")))
}

async fn get_ticket(client: &Client, existing: &str) -> Result<String> {
    if !existing.is_empty() {
        return Ok(existing.to_string());
    }
    let resp = api_post(client, ANONYMOUS, r#"{"dev_info":"{}"}"#, "", None, None).await?;
    let token = resp
        .data
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if token.is_empty() {
        bail!("empty anonymous token");
    }
    Ok(token)
}

async fn get_send_config(
    client: &Client,
    existing_token: &str,
    password: &str,
    total_size: u64,
    total_count: usize,
) -> Result<SendConfig> {
    let ticket = get_ticket(client, existing_token).await?;

    let time_resp = client
        .get(TIME_TOKEN)
        .headers(wss_headers())
        .send()
        .await
        .context("time token")?;
    let time_body: ApiResp = time_resp.json().await.context("parse time")?;
    if time_body.message != "success" {
        bail!("failed get timeToken");
    }
    let ts = time_body
        .data
        .get("time")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("no time field"))?
        .to_string();

    // Go encoding/json sorts map keys — a-code is MD5 over exact body bytes.
    let add_str = wss_addsend_body(password, total_size, total_count);
    let a_code = wss_sign(&ts, &ticket, add_str.as_bytes())?;
    let add = api_post(
        client,
        ADD_SEND,
        &add_str,
        &ticket,
        Some(&ts),
        Some(&a_code),
    )
    .await?;

    let mut conf: SendConfig = serde_json::from_value(add.data).context("parse addsend")?;
    conf.token = ticket.clone();

    let up_body = json!({
        "boxid": conf.bid,
        "preid": conf.ufileid,
        "linkid": conf.tid,
        "utype": "sendcopy",
        "originUpid": "",
        "length": total_size,
        "count": total_count,
    })
    .to_string();
    let up = api_post(client, GET_UP_ID, &up_body, &ticket, None, None).await?;
    conf.upload_id = up
        .data
        .get("upId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if conf.upload_id.is_empty() {
        bail!("empty upload id");
    }
    Ok(conf)
}

async fn put_part(
    client: &Client,
    conf: &SendConfig,
    name: &str,
    part: u64,
    block: usize,
    content: &[u8],
) -> Result<()> {
    let body = json!({
        "ispart": true,
        "fname": name,
        "partnu": part,
        "fsize": block,
        "upId": conf.upload_id,
    })
    .to_string();

    let mut last = None;
    for _ in 0..4 {
        match api_post(client, GET_UP_URL, &body, &conf.token, None, None).await {
            Ok(ticket) => {
                let url = ticket
                    .data
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if url.is_empty() {
                    last = Some(anyhow::anyhow!("empty put url"));
                    continue;
                }
                match client
                    .put(&url)
                    .header("content-type", "application/octet-stream")
                    .body(content.to_vec())
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let _ = resp.bytes().await;
                        return Ok(());
                    }
                    Err(e) => last = Some(e.into()),
                }
            }
            Err(e) => last = Some(e),
        }
        sleep(Duration::from_millis(300)).await;
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("put part {part} failed")))
}

async fn finish_parts(client: &Client, conf: &SendConfig, name: &str) -> Result<()> {
    let body = json!({
        "ispart": true,
        "fname": name,
        "location": {
            "boxid": conf.bid,
            "preid": conf.ufileid,
        },
        "upId": conf.upload_id,
    })
    .to_string();
    let resp = api_post(client, COMPLETE, &body, &conf.token, None, None).await?;
    if resp.message != "success" {
        bail!("upload failed returns: {}", resp.message);
    }
    Ok(())
}

async fn complete_upload(client: &Client, conf: &SendConfig, quiet: bool) -> Result<String> {
    let body = json!({ "processId": conf.upload_id }).to_string();
    loop {
        match api_post(client, PROCESS, &body, &conf.token, None, None).await {
            Ok(resp) => {
                let r = resp
                    .data
                    .get("rst")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if r == "success" {
                    break;
                }
            }
            Err(_) => {}
        }
        sleep(Duration::from_secs(1)).await;
    }

    let fin = json!({
        "bid": conf.bid,
        "ufileid": conf.ufileid,
        "tid": conf.tid,
    })
    .to_string();
    let resp = api_post(client, FINISH, &fin, &conf.token, None, None).await?;
    if resp.message != "success" {
        bail!("status != success");
    }
    // finish payload is mostly URLs; don't require bid/tid fields.
    let public_url = resp
        .data
        .get("public_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let manage_url = resp
        .data
        .get("mgr_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !quiet && !manage_url.is_empty() {
        eprintln!("manage: {manage_url}");
    }
    if public_url.is_empty() {
        bail!("empty public url; body={}", resp.data);
    }
    Ok(public_url)
}
