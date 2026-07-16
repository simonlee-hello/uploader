use std::fs;
use std::path::{Path, PathBuf};

pub struct UserConfig {
    pub backend: Option<String>,
    pub auto: bool,
}

pub fn config_dir() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("UPLOADER_CONFIG_DIR") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    #[cfg(windows)]
    {
        if let Ok(app) = std::env::var("APPDATA") {
            return Some(Path::new(&app).join("uploader"));
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(Path::new(&xdg).join("uploader"));
        }
    }
    dirs_home().map(|h| h.join(".config").join("uploader"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn load_user_config() -> UserConfig {
    let mut cfg = UserConfig {
        backend: None,
        auto: false,
    };
    let Some(dir) = config_dir() else {
        return cfg;
    };
    let Ok(text) = fs::read_to_string(dir.join("config")) else {
        return cfg;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim() {
            "backend" => cfg.backend = Some(v.trim().to_string()),
            "auto" => {
                let v = v.trim();
                cfg.auto = v.eq_ignore_ascii_case("true") || v == "1";
            }
            _ => {}
        }
    }
    cfg
}

pub fn resolve_default_backend() -> String {
    if let Ok(v) = std::env::var("UPLOADER_BACKEND") {
        let v = v.trim();
        if !v.is_empty() {
            return v.to_string();
        }
    }
    let cfg = load_user_config();
    if let Some(b) = cfg.backend {
        if !b.is_empty() {
            return b;
        }
    }
    if let Some(last) = read_last_backend() {
        return last;
    }
    "temp".to_string()
}

pub fn read_last_backend() -> Option<String> {
    let dir = config_dir()?;
    let s = fs::read_to_string(dir.join("last-backend")).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

pub fn save_last_backend(name: &str) {
    if name.is_empty() {
        return;
    }
    let Some(dir) = config_dir() else {
        return;
    };
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join("last-backend"), format!("{name}\n"));
}

pub fn config_auto_enabled() -> bool {
    load_user_config().auto
}
