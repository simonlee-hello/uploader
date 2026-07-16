use anyhow::{bail, Result};
use regex::Regex;
use std::sync::OnceLock;

fn size_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(\d+(?:\.\d+)?)\s*([kmgt]?b)?").unwrap())
}

/// Parse sizes like "100MB", "4GB", "5GB anon". Empty/none/- => 0 (unlimited).
pub fn parse_byte_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("none") || s == "-" {
        return Ok(0);
    }
    let caps = size_re()
        .captures(s)
        .ok_or_else(|| anyhow::anyhow!("invalid size {s:?}"))?;
    let n: f64 = caps[1].parse()?;
    let unit = caps
        .get(2)
        .map(|m| m.as_str().to_ascii_uppercase())
        .unwrap_or_else(|| "B".to_string());
    let mul = match unit.as_str() {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => bail!("invalid size unit in {s:?}"),
    };
    Ok((n * mul) as u64)
}

pub fn format_byte_size(n: u64) -> String {
    if n == 0 {
        return "unlimited".into();
    }
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    let n = n as f64;
    if n >= TB {
        format!("{:.1}TB", n / TB)
    } else if n >= GB {
        format!("{:.1}GB", n / GB)
    } else if n >= MB {
        format!("{:.1}MB", n / MB)
    } else if n >= KB {
        format!("{:.1}KB", n / KB)
    } else {
        format!("{n}B")
    }
}

pub fn check_upload_size(name: &str, size: u64, limit: u64, backend: &str) -> Result<()> {
    if limit == 0 || size <= limit {
        return Ok(());
    }
    let name = if name.is_empty() { "file" } else { name };
    bail!(
        "{name} is {}, backend {backend} limit is {} — abort before upload",
        format_byte_size(size),
        format_byte_size(limit)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sizes() {
        assert_eq!(parse_byte_size("100MB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_byte_size("5GB anon").unwrap(), 5 * 1024 * 1024 * 1024);
        assert_eq!(parse_byte_size("none").unwrap(), 0);
    }

    #[test]
    fn check_limit() {
        assert!(check_upload_size("a", 50 << 20, 100 << 20, "tmpf").is_ok());
        assert!(check_upload_size("a", 150 << 20, 100 << 20, "tmpf").is_err());
    }
}
