use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

use crate::config::resolve_default_backend;
use crate::crypto;
use crate::probe;
use crate::registry::{self, BackendStatus};
use crate::upload::{self, UploadOptions};

#[derive(Parser, Debug)]
#[command(
    name = "uploader",
    version = "0.1.0-rust",
    about = "Multi-backend file uploader (Rust rewrite)",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Backend name
    #[arg(short = 'b', long = "backend", global = true)]
    pub backend: Option<String>,

    /// Encrypt before upload
    #[arg(short = 'e', long = "encrypt", global = true)]
    pub encrypt: bool,

    /// Encryption key
    #[arg(short = 'k', long = "key", alias = "encrypt-key", global = true)]
    pub key: Option<String>,

    /// Quiet: links on stdout only
    #[arg(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,

    /// Print link only (same progress mute as quiet)
    #[arg(long = "silent", global = true)]
    pub silent: bool,

    /// Disable progress bar
    #[arg(long = "no-progress", global = true)]
    pub no_progress: bool,

    /// Recursive: upload each file under directory
    #[arg(short = 'r', long = "recursive", global = true)]
    pub recursive: bool,

    /// Single folder mode (gof/wss multi-file one link)
    #[arg(short = 's', long = "single", global = true)]
    pub single: bool,

    /// Share password (wss/fic)
    #[arg(long = "password", global = true)]
    pub password: Option<String>,

    /// WSS cookie / token
    #[arg(long = "cookie", global = true)]
    pub cookie: Option<String>,

    /// 1fichier FTP mode
    #[arg(long = "ftp", global = true)]
    pub ftp: bool,

    /// With -b: still probe+failover (no -b already means auto)
    #[arg(long = "auto", global = true)]
    pub auto: bool,

    /// Allow flaky/down backends
    #[arg(long = "force", global = true)]
    pub force: bool,

    /// HTTP timeout seconds (0 = none)
    #[arg(long = "http-timeout", default_value_t = 600, global = true)]
    pub http_timeout: u64,

    /// Append links to file
    #[arg(short = 'o', long = "result", global = true)]
    pub result: Option<PathBuf>,

    /// Verbose
    #[arg(short = 'v', long = "verbose", global = true)]
    pub verbose: bool,

    /// Files / directories to upload (when no subcommand)
    #[arg(global = true)]
    pub files: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List backends
    Backends,
    /// Probe backend availability
    Probe {
        #[arg(long)]
        all: bool,
        #[arg(long, default_value_t = 3)]
        parallel: usize,
        #[arg(long, default_value_t = 45.0)]
        timeout: f64,
        backends: Vec<String>,
    },
    /// Encrypt files locally
    Encrypt {
        #[arg(short = 'k', long = "key", alias = "encrypt-key")]
        key: Option<String>,
        #[arg(short = 'o', long = "output", alias = "out", default_value = ".")]
        output: PathBuf,
        #[arg(short = 'f', long = "force")]
        force: bool,
        files: Vec<PathBuf>,
    },
    /// Decrypt files locally
    Decrypt {
        #[arg(short = 'k', long = "key", alias = "encrypt-key")]
        key: Option<String>,
        #[arg(short = 'o', long = "output", alias = "out", default_value = ".")]
        output: PathBuf,
        #[arg(short = 'f', long = "force")]
        force: bool,
        files: Vec<PathBuf>,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Some(Commands::Backends) => {
                print!("{}", registry::format_table());
                Ok(())
            }
            Some(Commands::Probe {
                all,
                parallel,
                timeout,
                backends,
            }) => {
                let mut backends = backends;
                // Global `files` can steal trailing names when mixed with flags.
                for f in &self.files {
                    let s = f.display().to_string();
                    if !backends.iter().any(|b| b == &s) {
                        backends.push(s);
                    }
                }
                probe::run(backends, all, parallel, timeout).await
            }
            Some(Commands::Encrypt {
                key,
                output,
                force,
                files,
            }) => {
                if files.is_empty() {
                    bail!("usage: uploader encrypt [-k pass] [-o path] <file>");
                }
                for f in files {
                    crypto::encrypt_file(&f, &output, key.as_deref().unwrap_or(""), force)?;
                }
                Ok(())
            }
            Some(Commands::Decrypt {
                key,
                output,
                force,
                files,
            }) => {
                if files.is_empty() {
                    bail!("usage: uploader decrypt -k pass [-o path] <file>");
                }
                let key = key.as_deref().unwrap_or("");
                if key.is_empty() {
                    bail!("key required");
                }
                for f in files {
                    crypto::decrypt_file(&f, &output, key, force)?;
                }
                Ok(())
            }
            None => self.run_upload().await,
        }
    }

    async fn run_upload(self) -> Result<()> {
        if self.files.is_empty() {
            print_help();
            return Ok(());
        }

        let quiet = self.quiet || self.silent;
        // No -b → auto failover. With -b → pin (unless --auto).
        let pinned = self
            .backend
            .clone()
            .filter(|s| !s.trim().is_empty());
        let auto = pinned.is_none() || self.auto;
        let primary = pinned.unwrap_or_else(resolve_default_backend);

        let opts = UploadOptions {
            primary,
            files: self.files,
            encrypt: self.encrypt,
            key: self.key.unwrap_or_default(),
            quiet,
            no_progress: self.no_progress || quiet,
            recursive: self.recursive,
            auto,
            force: self.force,
            single: self.single,
            password: self.password.unwrap_or_default(),
            cookie: self.cookie.unwrap_or_default(),
            ftp: self.ftp,
            http_timeout_secs: self.http_timeout,
            result_file: self.result,
            verbose: self.verbose && !quiet,
        };

        upload::run(opts).await
    }
}

fn print_help() {
    println!(
        r#"uploader 0.1.0-rust — multi-backend file uploader (Rust rewrite)

Usage:
  uploader <file...>                 # auto: probe then upload via fastest fitting backend
  uploader -b <backend> <file...>    # pin one backend
  uploader backends
  uploader probe [-all] [backend...]
  uploader encrypt|decrypt [options] <file...>

Examples:
  uploader ./file
  uploader -q ./mydir
  uploader -b lit ./file
  uploader -b gof -s ./a ./b
  uploader -b lit -e -k pass ./file

Backends:
{table}
Flags:
  -b, --backend       pin backend (omit = auto: probe then upload)
  -e, --encrypt       encrypt before upload
  -k, --key           encryption key
  -s, --single        multi-file one link (gof/wss)
  --password          share password (wss/fic)
  --ftp               1fichier FTP mode
  -q, --quiet         headless: links on stdout only
  -r, --recursive     upload each file under a directory
  --auto              with -b: still probe+failover
  --force             allow flaky/down backends
  --http-timeout SEC  HTTP timeout (default 600)

Plan: docs/rust-refactor-plan.md
"#,
        table = registry::format_table()
    );
}

pub fn check_backend_allowed(name: &str, force: bool) -> Result<()> {
    let info = registry::find(name).ok_or_else(|| anyhow::anyhow!("unknown backend {name:?}"))?;
    match info.status {
        BackendStatus::Down if !force => {
            bail!(
                "backend {} is down ({}); use --force to try anyway",
                info.name,
                info.note
            );
        }
        BackendStatus::Flaky if !force => {
            bail!(
                "backend {} is flaky ({}); use --force to try anyway",
                info.name,
                info.note
            );
        }
        _ => Ok(()),
    }
}
