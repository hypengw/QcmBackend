use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use openssl::hash::Hasher;
pub use openssl::hash::MessageDigest;
pub use openssl::pkey;
pub use openssl::rsa::{Padding, Rsa};
pub use openssl::symm::Cipher;
use openssl::symm::{Crypter, Mode};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn encrypt(cipher: Cipher, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut encrypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
    let block_size = cipher.block_size();
    let mut output = vec![0; data.len() + block_size];

    let mut count = encrypter.update(data, &mut output)?;
    count += encrypter.finalize(&mut output[count..])?;
    output.truncate(count);

    Ok(output)
}

pub fn decrypt(cipher: Cipher, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut decrypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
    let block_size = cipher.block_size();
    let mut output = vec![0; data.len() + block_size];

    let mut count = decrypter.update(data, &mut output)?;
    count += decrypter.finalize(&mut output[count..])?;
    output.truncate(count);

    Ok(output)
}

// with 64 '\n'
pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
    let block = BASE64.encode(data);
    let block_bytes = block.as_bytes();
    let len = block.len();
    let block_len = len / 64;
    let last_line_len = len % 64;
    let fin = {
        let end = (last_line_len > 0) as usize;
        len + block_len + end
    };
    let mut out: Vec<u8> = Vec::new();
    out.resize(fin, 0);

    for i in 0..block_len {
        let out_prefix = i * 65;
        let in_prefix = i * 64;
        for j in 0..64 {
            out[out_prefix + j] = block_bytes[in_prefix + j];
        }
        out[out_prefix + 64] = '\n' as u8;
    }
    if last_line_len > 0 {
        let out_prefix = block_len * 65;
        let in_prefix = block_len * 64;
        for i in 0..last_line_len {
            out[out_prefix + i] = block_bytes[in_prefix + i];
        }
        out[out_prefix + last_line_len] = '\n' as u8;
    }
    Ok(out)
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    Ok(BASE64.decode(data)?)
}

pub fn digest(digest_type: MessageDigest, data: &[u8]) -> Result<Vec<u8>> {
    let mut hasher = Hasher::new(digest_type)?;
    hasher.update(data)?;
    Ok(hasher.finish()?.to_vec())
}

pub mod hex {
    pub fn encode_low(data: &[u8]) -> Vec<u8> {
        hex::encode(data).into_bytes()
    }

    pub fn encode_up(data: &[u8]) -> Vec<u8> {
        hex::encode_upper(data).into_bytes()
    }
}
