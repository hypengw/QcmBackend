use rand::Rng;
use std::error::Error;
use openssl::symm::Cipher;
use openssl::hash::MessageDigest;
use openssl::rsa::Padding;
use super::*;

const AES_IV: &[u8] = b"0102030405060708";
const AES_KEY: &[u8] = b"0CoJUm6Qyw8W8jud";
const RSA_PUB_KEY: &[u8] = b"-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgtQn2JZ34ZC28NWYpAUd98iZ37BUrX/aKzmFbt7cl
FSs6sXqHauqKWqdtLkF2KexO40H1YTX8z2lSgBBOAxLsvaklV8k4cBFK9snQXE9/DDaFt6Rr7iVZMldc
zhC0JNgTz+SHXT6CBHuX3e9SdB1Ua44oncaTWz7OBGLbCiK45wIDAQAB
-----END PUBLIC KEY-----";
const BASE62: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const EAPI_KEY: &[u8] = b"e82ckenh8dichen8";

pub struct Crypto {
    rsa: RsaKey,
}

impl Crypto {
    pub fn new() -> Result<Self> {
        let rsa = RsaKey::from_pem(RSA_PUB_KEY)?;
        Ok(Self { rsa })
    }

    pub fn weapi(&self, data: &[u8]) -> Result<String> {
        let mut rng = rand::thread_rng();
        let mut sec_key = [0u8; 16];
        for b in &mut sec_key {
            *b = BASE62[rng.gen::<usize>() % 62];
        }

        let params = encrypt(Cipher::aes_128_cbc(), AES_KEY, AES_IV, data)
            .and_then(|data| encode(&data))
            .and_then(|data| encrypt(Cipher::aes_128_cbc(), &sec_key, AES_IV, &data))
            .and_then(|data| encode(&data))?;

        let mut sec_key_padding = [0u8; 128];
        sec_key_padding[..16].copy_from_slice(&sec_key);
        sec_key_padding.reverse();

        let enc_sec_key = self.rsa.encrypt(Padding::NONE, &sec_key_padding)
            .map(|data| hex::encode_low(&data))?;

        Ok(format!(
            "params={}&encSecKey={}", 
            String::from_utf8(params)?,
            String::from_utf8(enc_sec_key)?
        ))
    }

    pub fn eapi(&self, url: &str, data: &[u8]) -> Result<String> {
        let message = format!("nobody{}use{}md5forencrypt", url, String::from_utf8_lossy(data));
        
        let params = digest(MessageDigest::md5(), message.as_bytes())
            .map(|hash| hex::encode_low(&hash))
            .map(|hash| {
                format!("{}-36cd479b6b5-{}-36cd479b6b5-{}", 
                    url,
                    String::from_utf8_lossy(data),
                    String::from_utf8_lossy(&hash)
                ).into_bytes()
            })
            .and_then(|data| encrypt(Cipher::aes_128_ecb(), EAPI_KEY, AES_IV, &data))
            .map(|data| hex::encode_up(&data))
            .map(|data| format!("params={}", String::from_utf8_lossy(&data)))?;

        Ok(params)
    }
}
