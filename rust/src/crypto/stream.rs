use aes::Aes256;
use anyhow::{bail, Result};
use cbc::{Decryptor, Encryptor};
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use rand::RngCore;

const MAGIC: &[u8] = b"UP01";
const BLOCK: usize = 16;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

/// PKCS7-style pad of `src` to `block_size` (same as Go crypto.Padding).
pub fn pad_key(src: &[u8], block_size: usize) -> Vec<u8> {
    let padding = block_size - src.len() % block_size;
    let mut out = src.to_vec();
    out.extend(std::iter::repeat(padding as u8).take(padding));
    out
}

pub fn normalize_key(key: &str, generate_if_empty: bool) -> Result<(String, String)> {
    let (display, bytes) = normalize_key_bytes(key, generate_if_empty)?;
    // Display is the user-facing key; normalized string is lossy for non-utf8 pads.
    let normalized = String::from_utf8_lossy(&bytes).into_owned();
    Ok((display, normalized))
}

/// Returns (display_key, key_bytes[32])
pub fn normalize_key_bytes(key: &str, generate_if_empty: bool) -> Result<(String, [u8; 32])> {
    let mut display = key.to_string();
    let mut raw = key.as_bytes().to_vec();
    if raw.is_empty() || raw.len() > 32 {
        if !generate_if_empty {
            bail!("key required");
        }
        display = gen_rand_string(16);
        raw = display.as_bytes().to_vec();
    }
    if raw.len() < 32 {
        raw = pad_key(&raw, 32);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw[..32]);
    Ok((display, out))
}

pub fn calc_encrypt_size(size: u64) -> u64 {
    let pad = BLOCK as u64 - (size % BLOCK as u64);
    MAGIC.len() as u64 + BLOCK as u64 + size + pad
}

pub fn encrypt_bytes(plain: &[u8], key: &str) -> Result<Vec<u8>> {
    let (_, key_bytes) = normalize_key_bytes(key, false)?;
    encrypt_with_key(plain, &key_bytes)
}

pub fn encrypt_with_key(plain: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let mut iv = [0u8; BLOCK];
    rand::thread_rng().fill_bytes(&mut iv);

    let cipher = Aes256CbcEnc::new(key.into(), &iv.into());
    let ciphertext = cipher.encrypt_padded_vec_mut::<Pkcs7>(plain);

    let mut out = Vec::with_capacity(MAGIC.len() + BLOCK + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_bytes(data: &[u8], key: &str) -> Result<Vec<u8>> {
    let (_, key_bytes) = normalize_key_bytes(key, false)?;
    decrypt_with_key(data, &key_bytes)
}

pub fn decrypt_with_key(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if data.len() < BLOCK {
        bail!("ciphertext too short");
    }

    if data.starts_with(MAGIC) {
        let body = &data[MAGIC.len()..];
        if body.len() < BLOCK * 2 || body.len() % BLOCK != 0 {
            bail!("invalid modern ciphertext length");
        }
        let iv = &body[..BLOCK];
        let ct = &body[BLOCK..];
        let cipher = Aes256CbcDec::new(key.into(), iv.into());
        return cipher
            .decrypt_padded_vec_mut::<Pkcs7>(ct)
            .map_err(|e| anyhow::anyhow!("decrypt: {e}"));
    }

    if data.len() % BLOCK != 0 {
        bail!("invalid ciphertext length");
    }
    let iv = [b'7'; BLOCK];
    let cipher = Aes256CbcDec::new(key.into(), &iv.into());
    cipher
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .map_err(|e| anyhow::anyhow!("decrypt: {e}"))
}

fn gen_rand_string(n: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| ALPHABET[rng.next_u32() as usize % ALPHABET.len()] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = "k".repeat(32);
        let raw = b"hello rust encrypt payload!!xx";
        let enc = encrypt_bytes(raw, &key).unwrap();
        assert_eq!(enc.len() as u64, calc_encrypt_size(raw.len() as u64));
        let dec = decrypt_bytes(&enc, &key).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn short_key_pad() {
        let (d, kb) = normalize_key_bytes("abc", true).unwrap();
        assert_eq!(d, "abc");
        assert_eq!(kb.len(), 32);
    }
}
