use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout;

use crate::backends::{self, CreateOpts, UploadItem};
use crate::http;
use crate::registry::{self, BackendInfo, BackendStatus};

struct ProbeResult {
    name: String,
    ok: bool,
    latency: Duration,
    link: String,
    err: String,
    skipped: bool,
}

pub async fn run(backends: Vec<String>, all: bool, parallel: usize, timeout_secs: f64) -> Result<()> {
    let parallel = parallel.max(1);
    let timeout_dur = Duration::from_secs_f64(timeout_secs.max(1.0));

    let targets = select_targets(&backends, all);
    if targets.is_empty() {
        bail!("no backends");
    }

    let probe_path = std::env::temp_dir().join(format!(
        "uploader-probe-{}.txt",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&probe_path, b"uploader probe\n")?;
    let probe_path_cleanup = probe_path.clone();

    eprintln!(
        "probing {} backend(s), parallel={parallel} timeout={:.0}s\n",
        targets.len(),
        timeout_secs
    );

    let client = http::build_client(timeout_secs.ceil() as u64 + 30)?;
    let sem = Arc::new(Semaphore::new(parallel));
    let print_mu = Arc::new(Mutex::new(()));
    let mut handles = Vec::new();

    for info in targets {
        let sem = sem.clone();
        let client = client.clone();
        let path = probe_path.clone();
        let print_mu = print_mu.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            let res = probe_one(&client, info, &path, timeout_dur).await;
            let status = if res.skipped {
                "SKIP"
            } else if res.ok {
                "OK"
            } else {
                "FAIL"
            };
            let extra = if res.ok {
                short_link(&res.link)
            } else {
                res.err.clone()
            };
            let _g = print_mu.lock().await;
            println!(
                "{status:<4} {:<6} {:>8}  {extra}",
                res.name,
                format_latency(res.latency)
            );
            res
        }));
    }

    let mut results = Vec::new();
    for h in handles {
        if let Ok(r) = h.await {
            results.push(r);
        }
    }
    let _ = fs::remove_file(probe_path_cleanup);

    results.sort_by(|a, b| {
        b.ok
            .cmp(&a.ok)
            .then_with(|| (!a.skipped).cmp(&(!b.skipped)))
            .then_with(|| {
                if a.ok && b.ok {
                    a.latency.cmp(&b.latency)
                } else {
                    a.name.cmp(&b.name)
                }
            })
    });

    eprintln!("\nsummary (prefer top successes):");
    println!("{:<6} {:<6} {:>8}  DETAIL", "NAME", "RESULT", "TIME");
    for r in &results {
        let res = if r.skipped {
            "skip"
        } else if r.ok {
            "ok"
        } else {
            "fail"
        };
        let detail = if r.ok {
            short_link(&r.link)
        } else {
            r.err.clone()
        };
        println!(
            "{:<6} {:<6} {:>8}  {detail}",
            r.name,
            res,
            format_latency(r.latency)
        );
    }

    for r in &results {
        if !r.ok {
            continue;
        }
        if let Some(info) = registry::find(&r.name) {
            if info.status == BackendStatus::Ok {
                eprintln!("\nrecommended: uploader -b {} <file>", r.name);
                return Ok(());
            }
        }
    }
    eprintln!("\nno working backend for this network");
    bail!("probe: no working backend");
}

fn select_targets(names: &[String], all: bool) -> Vec<&'static BackendInfo> {
    if !names.is_empty() {
        let mut out = Vec::new();
        for n in names {
            match registry::find(n) {
                Some(info) => out.push(info),
                None => eprintln!("unknown backend: {n}"),
            }
        }
        return out;
    }
    registry::BACKENDS
        .iter()
        .filter(|b| all || b.status == BackendStatus::Ok)
        .collect()
}

async fn probe_one(
    client: &reqwest::Client,
    info: &BackendInfo,
    file: &PathBuf,
    timeout_dur: Duration,
) -> ProbeResult {
    let mut res = ProbeResult {
        name: info.name.to_string(),
        ok: false,
        latency: Duration::ZERO,
        link: String::new(),
        err: String::new(),
        skipped: false,
    };
    if info.status == BackendStatus::Down {
        res.skipped = true;
        res.err = info.note.to_string();
        return res;
    }

    let start = Instant::now();
    let upload = async {
        let mut backend = backends::create(
            info.name,
            &CreateOpts {
                quiet: true,
                ..Default::default()
            },
        )?;
        let meta = fs::metadata(file)?;
        let item = UploadItem {
            path: file.clone(),
            name: "probe.txt".into(),
            size: meta.len(),
        };
        backend.init(client, &[item.clone()]).await?;
        let mut link = backend.upload(client, &item).await?;
        if link.is_empty() {
            if let Some(l) = backend.finish(client).await? {
                link = l;
            }
        }
        Ok::<String, anyhow::Error>(link)
    };

    match timeout(timeout_dur, upload).await {
        Ok(Ok(link)) => {
            res.latency = start.elapsed();
            res.ok = true;
            res.link = link;
        }
        Ok(Err(e)) => {
            res.latency = start.elapsed();
            res.err = truncate_err(&format!("{e:#}"), 72);
        }
        Err(_) => {
            res.latency = timeout_dur;
            res.err = format!("timeout >{:.0}s", timeout_dur.as_secs_f64());
        }
    }
    res
}

fn format_latency(d: Duration) -> String {
    if d.is_zero() {
        return "-".into();
    }
    if d < Duration::from_secs(1) {
        format!("{}ms", d.as_millis())
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

fn short_link(link: &str) -> String {
    let link = link.trim();
    if link.is_empty() {
        return "(no link)".into();
    }
    if link.len() > 64 {
        format!("{}...", &link[..61])
    } else {
        link.to_string()
    }
}

fn truncate_err(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= n {
        s
    } else {
        format!("{}...", &s[..n.saturating_sub(3)])
    }
}
