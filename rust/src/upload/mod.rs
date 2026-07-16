use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::backends::{self, CreateOpts, UploadItem};
use crate::cli::check_backend_allowed;
use crate::config;
use crate::crypto::{self};
use crate::http;
use crate::probe;
use crate::registry;
use crate::util::size::{check_upload_size, format_byte_size};
use crate::util::zip::zip_dir_temp;

pub struct UploadOptions {
    pub primary: String,
    pub files: Vec<PathBuf>,
    pub encrypt: bool,
    pub key: String,
    pub quiet: bool,
    pub no_progress: bool,
    pub recursive: bool,
    pub auto: bool,
    pub force: bool,
    pub single: bool,
    pub password: String,
    pub cookie: String,
    pub ftp: bool,
    pub http_timeout_secs: u64,
    pub result_file: Option<PathBuf>,
    pub verbose: bool,
}

struct Prepared {
    path: PathBuf,
    name: String,
    size: u64,
    cleanup: bool,
}

pub async fn run(opts: UploadOptions) -> Result<()> {
    let prepared = prepare_files(&opts)?;
    if prepared.is_empty() {
        bail!("no files to upload");
    }

    let mut key_bytes = [0u8; 32];
    if opts.encrypt {
        let (display, kb) = crypto::normalize_key_bytes(&opts.key, true)?;
        key_bytes = kb;
        if !opts.quiet {
            eprintln!("key: {display}");
        }
    }

    let mut upload_items: Vec<Prepared> = Vec::new();
    for p in prepared {
        if opts.encrypt {
            let mut plain = Vec::new();
            File::open(&p.path)?.read_to_end(&mut plain)?;
            let cipher = crypto::encrypt_with_key(&plain, &key_bytes)?;
            let tmp = tempfile::Builder::new()
                .prefix("enc-")
                .suffix(&format!("-{}", p.name))
                .tempfile()?;
            tmp.as_file().write_all(&cipher)?;
            let (f, path) = tmp.keep()?;
            drop(f);
            let name = backends::upload_name(Path::new(&p.name), true);
            if p.cleanup {
                let _ = fs::remove_file(&p.path);
            }
            upload_items.push(Prepared {
                path,
                name,
                size: cipher.len() as u64,
                cleanup: true,
            });
        } else {
            upload_items.push(p);
        }
    }

    let max_size = upload_items.iter().map(|i| i.size).max().unwrap_or(0);
    let candidates = if opts.auto {
        match probe::rank_for_upload(max_size, opts.force, opts.quiet).await {
            Ok(c) => c,
            Err(e) => {
                cleanup_all(&upload_items);
                return Err(e);
            }
        }
    } else {
        match build_candidates(&opts.primary, max_size, false, opts.force) {
            Ok(c) => c,
            Err(e) => {
                cleanup_all(&upload_items);
                return Err(e);
            }
        }
    };
    if candidates.is_empty() {
        cleanup_all(&upload_items);
        bail!("no backend available for this file size");
    }

    let client = http::build_client(opts.http_timeout_secs)?;
    let create_opts = CreateOpts {
        single: opts.single,
        password: opts.password.clone(),
        cookie: opts.cookie.clone(),
        quiet: opts.quiet,
        ftp: opts.ftp,
    };
    let mut last_err: Option<anyhow::Error> = None;

    for (i, name) in candidates.iter().enumerate() {
        let info = registry::find(name).context("backend")?;

        let mut size_fail = false;
        for item in &upload_items {
            if let Err(e) = check_upload_size(&item.name, item.size, info.max_bytes(), info.name) {
                let hint = size_hint(item.size, info.name);
                eprintln!("error: {e:#}");
                if !hint.is_empty() {
                    eprintln!("{hint}");
                }
                last_err = Some(e);
                size_fail = true;
                break;
            }
        }
        if size_fail {
            if opts.auto && i + 1 < candidates.len() {
                if !opts.quiet {
                    eprintln!("retry backend {}...", candidates[i + 1]);
                }
                continue;
            }
            cleanup_all(&upload_items);
            return Err(last_err.unwrap());
        }

        if !opts.quiet && opts.auto && i > 0 {
            eprintln!("retry backend {name}...");
        }

        let mut backend = match backends::create(name, &create_opts) {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(e);
                if opts.auto {
                    continue;
                }
                cleanup_all(&upload_items);
                return Err(last_err.unwrap());
            }
        };

        let items: Vec<UploadItem> = upload_items
            .iter()
            .map(|item| UploadItem {
                path: item.path.clone(),
                name: item.name.clone(),
                size: item.size,
            })
            .collect();

        if let Err(e) = backend.init(&client, &items).await {
            last_err = Some(e);
            if opts.auto {
                continue;
            }
            cleanup_all(&upload_items);
            return Err(last_err.unwrap());
        }

        let mut ok_all = true;
        for up in &items {
            match backend.upload(&client, up).await {
                Ok(link) => {
                    if !link.is_empty() {
                        println!("{link}");
                        if let Some(ref out) = opts.result_file {
                            append_result(out, &link)?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("upload {}: {e:#}", up.name);
                    last_err = Some(e);
                    ok_all = false;
                    break;
                }
            }
        }

        if ok_all {
            match backend.finish(&client).await {
                Ok(Some(link)) if !link.is_empty() => {
                    println!("{link}");
                    if let Some(ref out) = opts.result_file {
                        append_result(out, &link)?;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    last_err = Some(e);
                    ok_all = false;
                }
            }
        }

        if ok_all {
            config::save_last_backend(name);
            cleanup_all(&upload_items);
            return Ok(());
        }

        if !opts.auto {
            cleanup_all(&upload_items);
            return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("upload failed")));
        }
        if !opts.quiet {
            if let Some(ref e) = last_err {
                eprintln!("backend {name} failed: {e:#}");
            }
        }
    }

    cleanup_all(&upload_items);
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("all backends failed")))
}

fn cleanup_all(items: &[Prepared]) {
    for p in items {
        if p.cleanup {
            let _ = fs::remove_file(&p.path);
        }
    }
}

fn size_hint(size: u64, current: &str) -> String {
    let alts: Vec<_> = registry::backends_fitting(size)
        .into_iter()
        .filter(|a| *a != current)
        .take(6)
        .collect();
    if alts.is_empty() {
        return String::new();
    }
    format!("try: -b {}", alts.join(" | -b "))
}

fn build_candidates(primary: &str, max_size: u64, _auto: bool, force: bool) -> Result<Vec<String>> {
    check_backend_allowed(primary, force)?;
    let Some(info) = registry::find(primary) else {
        bail!("unknown backend {primary:?}");
    };
    let lim = info.max_bytes();
    if max_size > 0 && lim > 0 && max_size > lim {
        bail!(
            "{} exceeds backend {} limit",
            format_byte_size(max_size),
            info.name
        );
    }
    Ok(vec![info.name.to_string()])
}

fn prepare_files(opts: &UploadOptions) -> Result<Vec<Prepared>> {
    let mut out = Vec::new();
    let mut missing = Vec::new();

    for v in &opts.files {
        if !v.exists() {
            missing.push(v.display().to_string());
            continue;
        }
        let meta = fs::metadata(v)?;
        if meta.is_dir() && !opts.recursive {
            if !opts.quiet {
                eprintln!("packing {} ...", v.display());
            }
            let zip_path = zip_dir_temp(v)?;
            let zi = fs::metadata(&zip_path)?;
            if !opts.quiet {
                eprintln!(
                    "packed {} ({})",
                    zip_path.file_name().unwrap().to_string_lossy(),
                    format_byte_size(zi.len())
                );
            }
            let base = v
                .file_name()
                .map(|s| format!("{}.zip", s.to_string_lossy()))
                .unwrap_or_else(|| "archive.zip".into());
            out.push(Prepared {
                path: zip_path,
                name: base,
                size: zi.len(),
                cleanup: true,
            });
            continue;
        }

        for entry in WalkDir::new(v).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let name = backends::upload_name(&path, false);
            out.push(Prepared {
                path,
                name,
                size,
                cleanup: false,
            });
        }
    }

    if !missing.is_empty() {
        bail!("not found: {}", missing.join(", "));
    }
    Ok(out)
}

fn append_result(path: &Path, link: &str) -> Result<()> {
    use std::fs::OpenOptions;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("result file {}", path.display()))?;
    writeln!(f, "{link}")?;
    Ok(())
}
