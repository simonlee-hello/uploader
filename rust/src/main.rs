//! Uploader — Rust rewrite. See docs/rust-refactor-plan.md

mod backends;
mod cli;
mod config;
mod crypto;
mod http;
mod probe;
mod registry;
mod upload;
mod util;

use std::ffi::OsString;

use clap::Parser;
use cli::Cli;

fn normalize_argv() -> Vec<OsString> {
    // Go-style single-dash long flags → clap double-dash
    const MAP: &[(&str, &str)] = &[
        ("-auto", "--auto"),
        ("-force", "--force"),
        ("-silent", "--silent"),
        ("-encrypt", "--encrypt"),
        ("-backend", "--backend"),
        ("-recursive", "--recursive"),
        ("-quiet", "--quiet"),
        ("-result", "--result"),
        ("-http-timeout", "--http-timeout"),
        ("-no-progress", "--no-progress"),
        ("-password", "--password"),
        ("-cookie", "--cookie"),
        ("-ftp", "--ftp"),
        ("-single", "--single"),
        ("-encrypt-key", "--key"),
        ("-version", "--version"),
        ("-help", "--help"),
        ("-all", "--all"),
        ("-parallel", "--parallel"),
        ("-timeout", "--timeout"),
    ];
    std::env::args_os()
        .map(|a| {
            let s = a.to_string_lossy();
            for (from, to) in MAP {
                if s == *from || s.starts_with(&format!("{from}=")) {
                    return OsString::from(s.replacen(from, to, 1));
                }
            }
            a
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse_from(normalize_argv());
    if let Err(err) = cli.run().await {
        let msg = format!("{err:#}");
        // Size-limit path already printed error + hint.
        if !msg.contains("abort before upload") {
            eprintln!("{msg}");
        }
        std::process::exit(1);
    }
}
