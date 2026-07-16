mod des;
mod stream;

pub use stream::{
    decrypt_bytes, encrypt_bytes, encrypt_with_key, normalize_key, normalize_key_bytes,
};

#[allow(unused_imports)]
pub use des::{encrypt_des_cbc, wss_sign};

use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::util::progress::{finish_progress, ProgressWriter};

pub fn encrypt_file(src: &Path, output: &Path, key: &str, force: bool) -> Result<()> {
    let (display, norm) = normalize_key(key, true)?;
    if key.is_empty() {
        eprintln!("key: {display}");
    }
    let dest = resolve_encrypt_dest(src, output)?;
    if dest.exists() && !force {
        bail!("output exists: {} (use -force or -o PATH)", dest.display());
    }
    let meta = fs::metadata(src)?;
    let mut reader = File::open(src)?;
    let mut plain = Vec::with_capacity(meta.len() as usize);
    reader.read_to_end(&mut plain)?;
    let cipher = encrypt_bytes(&plain, &norm)?;
    let mut out = File::create(&dest)?;
    let mut pw = ProgressWriter::new(&mut out, cipher.len() as u64);
    pw.write_all(&cipher)?;
    finish_progress();
    println!("{} -> {}", src.display(), dest.display());
    Ok(())
}

pub fn decrypt_file(src: &Path, output: &Path, key: &str, force: bool) -> Result<()> {
    let (_, norm) = normalize_key(key, false)?;
    let dest = resolve_decrypt_dest(src, output)?;
    if dest.exists() && !force {
        bail!("output exists: {} (use -force or -o PATH)", dest.display());
    }
    let mut reader = File::open(src)?;
    let mut cipher = Vec::new();
    reader.read_to_end(&mut cipher)?;
    let plain = decrypt_bytes(&cipher, &norm)?;
    let mut out = File::create(&dest)?;
    let mut pw = ProgressWriter::new(&mut out, plain.len() as u64);
    pw.write_all(&plain)?;
    finish_progress();
    println!("{} -> {}", src.display(), dest.display());
    Ok(())
}

fn resolve_encrypt_dest(src: &Path, output: &Path) -> Result<PathBuf> {
    if output.is_dir() || output.as_os_str() == "." {
        let name = src.file_name().context("file name")?;
        Ok(output.join(format!("{}.encrypt", name.to_string_lossy())))
    } else {
        Ok(output.to_path_buf())
    }
}

fn resolve_decrypt_dest(src: &Path, output: &Path) -> Result<PathBuf> {
    if output.is_dir() || output.as_os_str() == "." {
        let name = src.file_name().context("file name")?.to_string_lossy();
        let stripped = name.strip_suffix(".encrypt").unwrap_or(&name);
        Ok(output.join(stripped))
    } else {
        Ok(output.to_path_buf())
    }
}
