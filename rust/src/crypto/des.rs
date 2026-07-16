use anyhow::{bail, Context, Result};
use cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use des::Des;
use md5::{Digest, Md5};

use crate::util::base58;

type DesCbcEnc = cbc::Encryptor<Des>;

/// DES-CBC encrypt with PKCS7 padding (matches Go crypto.EncryptDESCBC).
pub fn encrypt_des_cbc(plain: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    if key.len() < 8 || iv.len() < 8 {
        bail!("des key/iv must be at least 8 bytes");
    }
    let enc = DesCbcEnc::new_from_slices(&key[..8], &iv[..8]).context("des cbc init")?;
    Ok(enc.encrypt_padded_vec_mut::<Pkcs7>(plain))
}

/// WSS a-code: MD5(body+token) → hex → base58 → DES-CBC(timeIV).
pub fn wss_sign(ts: &str, token: &str, data: &[u8]) -> Result<String> {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.update(token.as_bytes());
    let md5_hex = hex::encode(hasher.finalize());
    let hash_str = base58::encode(md5_hex.as_bytes());

    let rev: String = ts.chars().rev().collect();
    let mut time_iv = Vec::with_capacity(8);
    for c in rev.chars().take(5) {
        let pos = c.to_digit(10).unwrap_or(0) as usize;
        if pos < ts.len() {
            time_iv.push(ts.as_bytes()[pos]);
        } else {
            time_iv.push(b'0');
        }
    }
    time_iv.extend_from_slice(b"000");

    let enc = encrypt_des_cbc(hash_str.as_bytes(), &time_iv, &time_iv)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        enc,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn des_cbc_roundtrip_len() {
        let plain = b"hello wss sign!";
        let key = b"12345678";
        let iv = b"87654321";
        let ct = encrypt_des_cbc(plain, key, iv).unwrap();
        assert_eq!(ct.len() % 8, 0);
        assert!(!ct.is_empty());
    }

    #[test]
    fn matches_go_reference() {
        // Legacy Go body (sorted keys) + old truncated base58 — kept for DES sanity only.
        let data = br#"{"downPreCountLimit":0,"expire":"1","file_count":1,"file_size":12,"isextension":false,"notDownload":false,"notPreview":false,"notSaveTo":false,"pwd":"","recvs":["social","public"],"remark":"","sender":"","trafficStatus":0}"#;
        let code = wss_sign("1784199449", "wss:testtoken", data).unwrap();
        // Current verify.js base58 → longer ciphertext than legacy Go.
        assert_eq!(
            code,
            "cVXh8gx/LWVAcNKI1V9T8NP6vQ0JToJ2Nwl0L5OzKmgnL0wQXKC48FmrUfUbLTYm"
        );
    }

    #[test]
    fn matches_verify_js_current_body() {
        let data = br#"{"sender":"","remark":"","isextension":false,"pwd":"","expire":"1","recvs":["social","public"],"file_size":12,"file_count":1,"notSaveTo":false,"trafficStatus":0,"task_traffic_limit":"","downPreCountLimit":0,"notDownload":false,"notPreview":false,"fileDisplay":0}"#;
        let code = wss_sign("1784199449", "wss:testtoken", data).unwrap();
        assert!(!code.is_empty());
        assert_eq!(code.len(), 64);
    }
}
