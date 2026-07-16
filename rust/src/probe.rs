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
use crate::util::size::format_byte_size;

pub struct ProbeResult {
    pub name: String,
    pub ok: bool,
    pub latency: Duration,
    pub link: String,
    pub err: String,
    pub skipped: bool,
}

pub async fn run(backends: Vec<String>, all: bool, parallel: usize, timeout_secs: f64) -> Result<()> {
    let parallel = parallel.max(1);
    let timeout_dur = Duration::from_secs_f64(timeout_secs.max(1.0));

    let targets = select_targets(&backends, all);
    if targets.is_empty() {
        bail!("no backends");
    }

    eprintln!(
        "probing {} backend(s), parallel={parallel} timeout={:.0}s\n",
        targets.len(),
        timeout_secs
    );

    let results = probe_all(&targets, parallel, timeout_dur, true).await?;
    let mut results = results;
    sort_probe_results(&mut results);

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

/// Probe size-fitting backends; return names sorted by latency (fastest first).
pub async fn rank_for_upload(max_size: u64, force: bool, quiet: bool) -> Result<Vec<String>> {
    let targets = select_targets_for_size(max_size, force);
    if targets.is_empty() {
        bail!("no backend available for this file size");
    }
    if !quiet {
        let size_label = if max_size == 0 {
            "?".into()
        } else {
            format_byte_size(max_size)
        };
        eprintln!(
            "auto: probing {} backend(s) (size ≤ {size_label})...",
            targets.len()
        );
    }

    let results = probe_all(&targets, 3, Duration::from_secs(45), !quiet).await?;
    let mut results = results;
    sort_probe_results(&mut results);

    let ranked: Vec<String> = results
        .iter()
        .filter(|r| r.ok)
        .map(|r| r.name.clone())
        .collect();
    if ranked.is_empty() {
        bail!("auto: no working backend (probe all failed)");
    }
    if !quiet {
        let lat = results
            .iter()
            .find(|r| r.ok && r.name == ranked[0])
            .map(|r| r.latency)
            .unwrap_or_default();
        eprintln!("auto: using {} ({})", ranked[0], format_latency(lat));
    }
    Ok(ranked)
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

fn select_targets_for_size(max_size: u64, force: bool) -> Vec<&'static BackendInfo> {
    registry::BACKENDS
        .iter()
        .filter(|b| {
            if b.status == BackendStatus::Down && !force {
                return false;
            }
            if b.status == BackendStatus::Flaky && !force {
                return false;
            }
            if b.status != BackendStatus::Ok && !force {
                return false;
            }
            let lim = b.max_bytes();
            !(max_size > 0 && lim > 0 && max_size > lim)
        })
        .collect()
}

async fn probe_all(
    targets: &[&'static BackendInfo],
    parallel: usize,
    timeout_dur: Duration,
    print_live: bool,
) -> Result<Vec<ProbeResult>> {
    let probe_path = std::env::temp_dir().join(format!(
        "uploader-probe-{}.txt",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&probe_path, b"uploader probe\n")?;
    let probe_path_cleanup = probe_path.clone();

    let client = http::build_client(timeout_dur.as_secs() + 30)?;
    let sem = Arc::new(Semaphore::new(parallel.max(1)));
    let print_mu = Arc::new(Mutex::new(()));
    let mut handles = Vec::new();

    for info in targets.iter().copied() {
        let sem = sem.clone();
        let client = client.clone();
        let path = probe_path.clone();
        let print_mu = print_mu.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            let res = probe_one(&client, info, &path, timeout_dur).await;
            if print_live {
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
                eprintln!(
                    "{status:<4} {:<6} {:>8}  {extra}",
                    res.name,
                    format_latency(res.latency)
                );
            }
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
    Ok(results)
}

fn sort_probe_results(results: &mut [ProbeResult]) {
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
