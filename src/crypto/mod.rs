use openssl::symm::{Cipher, Crypter, Mode};
use openssl::hash::{Hasher, MessageDigest};
use openssl::rsa::{Rsa, Padding};
use openssl::pkey::PKey;
use std::error::Error;

pub mod ncm;

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

pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
    Ok(base64::encode(data).into_bytes())
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    Ok(base64::decode(data)?)
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

pub struct RsaKey {
    key: PKey<openssl::pkey::Public>
}

impl RsaKey {
    pub fn from_pem(data: &[u8]) -> Result<Self> {
        let rsa = Rsa::public_key_from_pem(data)?;
        let key = PKey::from_rsa(rsa)?;
        Ok(Self { key })
    }

    pub fn encrypt(&self, padding: Padding, data: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0; self.key.size() as usize];
        let len = self.key.public_encrypt(data, &mut buf, padding)?;
        buf.truncate(len);
        Ok(buf)
    }
}
