use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Pack root_dir into a temp *.zip (Deflate best). Caller should delete the path.
pub fn zip_dir_temp(root_dir: &Path) -> Result<PathBuf> {
    let base = root_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let tmp = tempfile::Builder::new()
        .prefix(&format!("{base}-"))
        .suffix(".zip")
        .tempfile()
        .context("create temp zip")?;
    let (file, persisted) = tmp.keep().context("persist temp zip")?;
    drop(file);
    if let Err(e) = zip_dir(root_dir, &persisted) {
        let _ = fs::remove_file(&persisted);
        return Err(e);
    }
    Ok(persisted)
}

pub fn zip_dir(root_dir: &Path, dest_zip: &Path) -> Result<()> {
    let meta = fs::metadata(root_dir).with_context(|| format!("stat {}", root_dir.display()))?;
    if !meta.is_dir() {
        bail!("not a directory: {}", root_dir.display());
    }
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    let root = root_dir
        .canonicalize()
        .unwrap_or_else(|_| root_dir.to_path_buf());

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == root {
            continue;
        }
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let name = rel.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), opts)?;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        zip.start_file(&name, opts)?;
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_compresses() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("data");
        fs::create_dir_all(&sub).unwrap();
        let payload = vec![b'A'; 200 * 1024];
        fs::write(sub.join("big.txt"), &payload).unwrap();
        let out = dir.path().join("data.zip");
        zip_dir(&sub, &out).unwrap();
        let zi = fs::metadata(&out).unwrap().len();
        assert!(zi < payload.len() as u64, "zip={zi} raw={}", payload.len());
    }
}
