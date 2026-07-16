use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendStatus {
    Ok,
    Flaky,
    Down,
}

impl fmt::Display for BackendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Flaky => write!(f, "flaky"),
            Self::Down => write!(f, "down"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BackendInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub site: &'static str,
    pub limit: &'static str,
    pub status: BackendStatus,
    pub note: &'static str,
}

impl BackendInfo {
    pub fn max_bytes(&self) -> u64 {
        crate::util::size::parse_byte_size(self.limit).unwrap_or(0)
    }
}

pub static BACKENDS: &[BackendInfo] = &[
    BackendInfo {
        name: "temp",
        aliases: &["tempsh"],
        site: "https://temp.sh/",
        limit: "4GB",
        status: BackendStatus::Ok,
        note: "recommended",
    },
    BackendInfo {
        name: "tmpf",
        aliases: &["tempfiles", "tmpfiles"],
        site: "https://tmpfiles.org/",
        limit: "100MB",
        status: BackendStatus::Ok,
        note: "needs file ext; CF may reset",
    },
    BackendInfo {
        name: "lit",
        aliases: &["litterbox"],
        site: "https://litterbox.catbox.moe/",
        limit: "1GB",
        status: BackendStatus::Ok,
        note: "expires ~72h",
    },
    BackendInfo {
        name: "gof",
        aliases: &["gofile"],
        site: "https://gofile.io/",
        limit: "none",
        status: BackendStatus::Ok,
        note: "-s multi-file",
    },
    BackendInfo {
        name: "cnet",
        aliases: &["paste"],
        site: "https://paste.c-net.org/",
        limit: "50MB",
        status: BackendStatus::Ok,
        note: "rate/ip sensitive",
    },
    BackendInfo {
        name: "gg",
        aliases: &["downloadgg"],
        site: "https://download.gg/",
        limit: "25GB",
        status: BackendStatus::Ok,
        note: "-",
    },
    BackendInfo {
        name: "fic",
        aliases: &["1fichier"],
        site: "https://1fichier.com/",
        limit: "300GB",
        status: BackendStatus::Ok,
        note: "HTTP/--ftp; --password",
    },
    BackendInfo {
        name: "wss",
        aliases: &["wenshushu"],
        site: "https://wenshushu.cn/",
        limit: "5GB anon",
        status: BackendStatus::Ok,
        note: "chunked; zh Accept-Language",
    },
    BackendInfo {
        name: "cat",
        aliases: &["catbox"],
        site: "https://catbox.moe/",
        limit: "200MB",
        status: BackendStatus::Flaky,
        note: "anon often blocked",
    },
    BackendInfo {
        name: "bash",
        aliases: &["bashupload"],
        site: "https://bashupload.com/",
        limit: "50GB",
        status: BackendStatus::Flaky,
        note: "tls issues",
    },
    BackendInfo {
        name: "nil",
        aliases: &["null", "0x0"],
        site: "https://0x0.st/",
        limit: "512MB",
        status: BackendStatus::Down,
        note: "uploads disabled",
    },
];

pub fn find(name: &str) -> Option<&'static BackendInfo> {
    let n = name.trim().to_ascii_lowercase();
    BACKENDS.iter().find(|b| {
        b.name.eq_ignore_ascii_case(&n) || b.aliases.iter().any(|a| a.eq_ignore_ascii_case(&n))
    })
}

pub fn backends_fitting(size: u64) -> Vec<&'static str> {
    BACKENDS
        .iter()
        .filter(|b| b.status == BackendStatus::Ok)
        .filter(|b| {
            let lim = b.max_bytes();
            lim == 0 || size <= lim
        })
        .map(|b| b.name)
        .collect()
}

pub fn format_table() -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "  {:<6} {:<10} {:<6} {:<32} {}\n",
        "NAME", "LIMIT", "STATUS", "URL", "NOTES"
    ));
    for b in BACKENDS {
        let note = if b.note.is_empty() { "-" } else { b.note };
        out.push_str(&format!(
            "  {:<6} {:<10} {:<6} {:<32} {}\n",
            b.name, b.limit, b.status, b.site, note
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_alias() {
        assert_eq!(find("litterbox").unwrap().name, "lit");
        assert_eq!(find("TMPFILES").unwrap().name, "tmpf");
    }

    #[test]
    fn tmpf_limit() {
        assert_eq!(find("tmpf").unwrap().max_bytes(), 100 * 1024 * 1024);
    }
}
